import random
from pytest import fixture, mark, skip
from src.blockchain_api import BlockchainApi
from config.api_config import ApiConfig, NativeToken, MainchainAccount


@fixture(scope="session", autouse=True)
def reserve_cfg(config: ApiConfig) -> NativeToken:
    if not config.main_chain.native_token:
        skip("Native token is not configured")
    return config.main_chain.native_token


@fixture(scope="session")
def governance_authority(config: ApiConfig) -> MainchainAccount:
    return config.nodes_config.governance_authority


@fixture(scope="session")
def addresses(api: BlockchainApi):
    return api.partner_chains_node.smart_contracts.get_scripts().json["addresses"]


@mark.xdist_group(name="governance_action")
def test_create_reserve(api: BlockchainApi, reserve_cfg, governance_authority, addresses):
    native_token_balance = api.get_mc_balance(governance_authority.mainchain_address, reserve_cfg.token)
    assert native_token_balance > 0, "Native token not found. Have you minted it?"

    initial_deposit = 1000
    assert initial_deposit < native_token_balance, "Not enough tokens to create reserve"

    response = api.partner_chains_node.smart_contracts.reserve.create(
        v_function_hash=reserve_cfg.total_accrued_function_script_hash,
        initial_deposit=initial_deposit,
        token=reserve_cfg.token,
        payment_key=governance_authority.mainchain_key,
    )
    assert not response.stderr
    if "Reserve already exists with the same settings" in response.stdout:
        skip("Reserve already exists")
    assert response.transaction_id

    native_token_current_balance = api.get_mc_balance(governance_authority.mainchain_address, reserve_cfg.token)
    assert native_token_balance - initial_deposit == native_token_current_balance

    reserve_balance = api.get_mc_balance(addresses["ReserveValidator"], reserve_cfg.token)
    assert initial_deposit == reserve_balance


@mark.xdist_group(name="governance_action")
def test_release_funds(api: BlockchainApi, reserve_cfg, governance_authority, addresses):
    v_function_addr_utxo = api.cardano_cli.get_utxos(reserve_cfg.total_accrued_function_address)
    reference_utxo = next(iter(v_function_addr_utxo))

    circulation_before_release = api.get_mc_balance(addresses["IlliquidCirculationSupplyValidator"], reserve_cfg.token)
    reserve_before_release = api.get_mc_balance(addresses["ReserveValidator"], reserve_cfg.token)
    assert reserve_before_release > 0, "Reserve is empty"

    amount_to_release = random.randint(1, min(reserve_before_release, 100))
    response = api.partner_chains_node.smart_contracts.reserve.release(
        reference_utxo=reference_utxo, amount=amount_to_release, payment_key=governance_authority.mainchain_key
    )
    assert not response.stderr
    assert response.transaction_id

    circulation = api.get_mc_balance(addresses["IlliquidCirculationSupplyValidator"], reserve_cfg.token)
    reserve = api.get_mc_balance(addresses["ReserveValidator"], reserve_cfg.token)
    assert circulation_before_release + amount_to_release == circulation
    assert reserve_before_release - amount_to_release == reserve


@mark.xdist_group(name="governance_action")
def test_deposit_funds(api: BlockchainApi, reserve_cfg, governance_authority, addresses):
    native_token_before_deposit = api.get_mc_balance(governance_authority.mainchain_address, reserve_cfg.token)
    reserve_before_deposit = api.get_mc_balance(addresses["ReserveValidator"], reserve_cfg.token)
    assert native_token_before_deposit > 0, "Native token not found. Have you minted it?"

    amount_to_deposit = random.randint(1, min(native_token_before_deposit, 100))
    response = api.partner_chains_node.smart_contracts.reserve.deposit(
        amount=amount_to_deposit, payment_key=governance_authority.mainchain_key
    )
    assert not response.stderr
    assert response.transaction_id

    native_token = api.get_mc_balance(governance_authority.mainchain_address, reserve_cfg.token)
    assert native_token_before_deposit - amount_to_deposit == native_token
    reserve = api.get_mc_balance(addresses["ReserveValidator"], reserve_cfg.token)
    assert reserve_before_deposit + amount_to_deposit == reserve


@mark.xdist_group(name="governance_action")
def test_handover_reserve(api: BlockchainApi, reserve_cfg, governance_authority, addresses):
    circulation_before_handover = api.get_mc_balance(addresses["IlliquidCirculationSupplyValidator"], reserve_cfg.token)
    reserve_before_handover = api.get_mc_balance(addresses["ReserveValidator"], reserve_cfg.token)

    response = api.partner_chains_node.smart_contracts.reserve.handover(payment_key=governance_authority.mainchain_key)
    assert not response.stderr
    assert response.transaction_id

    reserve = api.get_mc_balance(addresses["ReserveValidator"], reserve_cfg.token)
    assert reserve == 0

    circulation = api.get_mc_balance(addresses["IlliquidCirculationSupplyValidator"], reserve_cfg.token)
    assert circulation_before_handover + reserve_before_handover == circulation
