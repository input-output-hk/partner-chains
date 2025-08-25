from pytest import fixture, mark
from config.api_config import ApiConfig
from src.blockchain_api import BlockchainApi
from src.cardano_cli import cbor_to_bech32, hex_to_bech32
from src.partner_chains_node.models import Reserve, VFunction
import json
import logging


def pytest_collection_modifyitems(items):
    for item in items:
        if "tests/reserve" in item.nodeid:
            item.add_marker(mark.reserve)
        if "observe" in item.nodeid:
            item.add_marker(
                mark.skipif(
                    item.config.security_param > 20,
                    reason="Observability tests would take too long to run with a high main chain security parameter",
                )
            )


@fixture(scope="session")
def governance_address(config: ApiConfig) -> str:
    return config.nodes_config.governance_authority.mainchain_address


@fixture(scope="session")
def payment_key(config: ApiConfig, governance_skey_with_cli):
    return config.nodes_config.governance_authority.mainchain_key


@fixture(scope="session")
def governance_skey_with_cardano_cli(config: ApiConfig, api: BlockchainApi, write_file):
    """
    Securely copy the governance authority's init skey (a secret key used by the smart-contracts to authorize admin
    operations) to a temporary file on the remote machine.
    The temporary file is deleted after the test completes.

    The skey is copied to a remote host only if both conditions are met:
    - you call this fixture in test or other fixture
    - tools.cardano_cli.runner.copy_secrets is set to true in the config file `<env>_stack.json`

    WARNING: This fixture copies secret file to a remote host and should be used with caution.
    """
    payment_key_path = config.nodes_config.governance_authority.mainchain_key_local_path
    if api.cardano_cli.run_command.copy_secrets:
        with open(payment_key_path, "r") as f:
            content = json.load(f)
            path = write_file(api.cardano_cli.run_command, content)
        return path
    else:
        return payment_key_path


@fixture(scope="session")
def governance_vkey_bech32(config: ApiConfig):
    vkey = config.nodes_config.governance_authority.mainchain_pub_key
    vkey_bech32 = hex_to_bech32(vkey, "addr_vk")
    return vkey_bech32


@fixture(scope="package")
def reserve(reserve_asset_id, v_function: VFunction) -> Reserve:
    reserve = Reserve(token=reserve_asset_id, v_function=v_function)
    return reserve


@fixture(scope="package")
def v_function(v_function_factory, config: ApiConfig):
    v_function_path = config.nodes_config.reserve.v_function_script_path
    v_function = v_function_factory(v_function_path)
    return v_function


@fixture(scope="package")
def minting_policy_filepath(api: BlockchainApi, governance_vkey_bech32, write_file):
    key_hash = api.cardano_cli.get_address_key_hash(governance_vkey_bech32)
    policy_script = {"keyHash": key_hash, "type": "sig"}
    policy_script_filepath = write_file(api.cardano_cli.run_command, policy_script)
    return policy_script_filepath


@fixture(scope="package")
def minting_policy_id(api: BlockchainApi, minting_policy_filepath):
    policy_id = api.cardano_cli.get_policy_id(minting_policy_filepath)
    return policy_id


@fixture(scope="package")
def reserve_asset_id(config: ApiConfig, minting_policy_id) -> str:
    asset_name = config.nodes_config.reserve.token_name
    asset_name_hex = asset_name.encode("utf-8").hex()
    policy_id = minting_policy_id
    return f"{policy_id}.{asset_name_hex}"


@fixture(scope="package")
def mint_token(
    governance_address: str,
    reserve_asset_id: str,
    transaction_input: str,
    minting_policy_filepath,
    api: BlockchainApi,
    governance_skey_with_cardano_cli,
):
    lovelace_amount = MIN_LOVELACE_FOR_TX - MIN_LOVELACE_TO_COVER_FEES

    def _mint_token(amount: int):
        logging.info(f"Minting {amount} native tokens...")
        _, tx_filepath = api.cardano_cli.build_mint_tx(
            tx_in=transaction_input(),
            address=governance_address,
            lovelace=lovelace_amount,
            amount=amount,
            asset_id=reserve_asset_id,
            policy_script_filepath=minting_policy_filepath,
        )

        signed_tx_filepath = api.cardano_cli.sign_transaction(
            tx_filepath=tx_filepath, signing_key=governance_skey_with_cardano_cli
        )

        result = api.cardano_cli.submit_transaction(signed_tx_filepath)
        return result

    return _mint_token


@fixture(scope="package")
def read_v_function_script_file():
    def _read_v_function_script_file(script_path):
        with open(script_path, "r") as file:
            v_function_script = json.loads(file.read())
        return v_function_script

    return _read_v_function_script_file


@fixture(scope="package")
def v_function_address(api: BlockchainApi):
    _, verification_key = api.cardano_cli.generate_payment_keys()
    bech32_vkey = cbor_to_bech32(verification_key["cborHex"], "addr_vk")
    address = api.cardano_cli.build_address(payment_vkey=bech32_vkey)
    return address


@fixture(scope="package")
def v_function_factory(
    api: BlockchainApi,
    read_v_function_script_file,
    write_file,
    v_function_address,
    attach_v_function_to_utxo,
    reference_utxo,
    wait_until,
    config: ApiConfig,
):
    def _v_function_factory(v_function_path):
        logging.info(f"Creating V-function from {v_function_path}...")
        v_function_script = read_v_function_script_file(v_function_path)
        v_function_cbor = v_function_script["cborHex"]
        script_path = write_file(api.cardano_cli.run_command, v_function_script)
        script_hash = api.cardano_cli.get_policy_id(script_path)
        attach_v_function_to_utxo(v_function_address, script_path)
        utxo = wait_until(reference_utxo, v_function_address, v_function_cbor, timeout=config.timeouts.main_chain_tx)
        v_function = VFunction(
            cbor=v_function_cbor,
            script_path=script_path,
            script_hash=script_hash,
            address=v_function_address,
            reference_utxo=utxo,
        )
        logging.info(f"V-function successfully created: {v_function}")
        return v_function

    return _v_function_factory


MIN_LOVELACE_FOR_TX = 20_000_000
MIN_LOVELACE_TO_COVER_FEES = 10_000_000


@fixture(scope="package")
def transaction_input(governance_address: str, api: BlockchainApi):

    def _transaction_input():
        utxo_dict = api.cardano_cli.get_utxos(governance_address)
        tx_in = next(filter(lambda utxo: utxo_dict[utxo]["value"]["lovelace"] > MIN_LOVELACE_FOR_TX, utxo_dict), None)
        return tx_in

    return _transaction_input


@fixture(scope="package")
def attach_v_function_to_utxo(
    transaction_input, governance_address, governance_skey_with_cardano_cli, api: BlockchainApi
):
    def _attach_v_function_to_utxo(address, filepath):
        logging.info(f"Attaching V-function to {address}...")
        lovelace_amount = MIN_LOVELACE_FOR_TX - MIN_LOVELACE_TO_COVER_FEES
        _, raw_tx_filepath = api.cardano_cli.build_tx_with_reference_script(
            tx_in=transaction_input(),
            address=address,
            lovelace=lovelace_amount,
            reference_script_file=filepath,
            change_address=governance_address,
        )

        signed_tx_filepath = api.cardano_cli.sign_transaction(
            tx_filepath=raw_tx_filepath, signing_key=governance_skey_with_cardano_cli
        )

        result = api.cardano_cli.submit_transaction(signed_tx_filepath)
        return result

    return _attach_v_function_to_utxo


@fixture(scope="package")
def reference_utxo(api: BlockchainApi):

    def _reference_utxo(v_function_address, cbor):
        utxo_dict = api.cardano_cli.get_utxos(v_function_address)
        reference_utxo = next(
            filter(lambda utxo: utxo_dict[utxo]["referenceScript"]["script"]["cborHex"] == cbor, utxo_dict), None
        )
        return reference_utxo

    return _reference_utxo
