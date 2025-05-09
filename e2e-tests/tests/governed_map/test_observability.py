from pytest import fixture, mark, skip
from src.blockchain_api import BlockchainApi
from src.cardano_cli import cbor_to_bech32
from conftest import string_to_hex_bytes
import logging

pytestmark = [mark.xdist_group(name="governance_action")]


@fixture(scope="session")
def sudo(api: BlockchainApi, secrets):
    sudo_config = secrets["wallets"]["sudo"]
    sudo = api.get_wallet(
        address=sudo_config["address"],
        public_key=sudo_config["public_key"],
        secret=sudo_config["secret_seed"],
        scheme=sudo_config["scheme"],
    )
    return sudo


@fixture(scope="session", autouse=True)
def get_governed_map_main_chain_scripts(api: BlockchainApi):
    result = api.get_governed_map_main_chain_scripts()
    logging.info(f"Current governed map main chain scripts: {result}")
    return result


GOVERNED_MAP_MAIN_CHAIN_SCRIPTS_ALREADY_SET = 1


@fixture(scope="session", autouse=True)
def set_validator_address(api: BlockchainApi, addresses, policy_ids, sudo, get_governed_map_main_chain_scripts):
    validator_address = get_governed_map_main_chain_scripts["validator_address"]
    asset_policy_id = get_governed_map_main_chain_scripts["asset_policy_id"]
    if validator_address == addresses["GovernedMapValidator"] and asset_policy_id == policy_ids["GovernedMap"]:
        return GOVERNED_MAP_MAIN_CHAIN_SCRIPTS_ALREADY_SET
    tx = api.set_governed_map_main_chain_scripts(addresses["GovernedMapValidator"], policy_ids["GovernedMap"], sudo)
    return tx


@fixture(scope="session", autouse=True)
def observe_governed_map_initialization(api: BlockchainApi, set_validator_address):
    if set_validator_address != GOVERNED_MAP_MAIN_CHAIN_SCRIPTS_ALREADY_SET:
        result = api.subscribe_governed_map_change()
        return result


class TestInitializeMap:
    def test_set_main_chain_scripts(self, request, set_validator_address):
        if set_validator_address == GOVERNED_MAP_MAIN_CHAIN_SCRIPTS_ALREADY_SET:
            skip(f"Governed map main chain scripts are already set correctly. Skipping test {request.node.nodeid}.")
        tx = set_validator_address
        assert tx._receipt.is_success, f"Failed to set new governed map address: {tx._receipt.error_message}"

    def test_observed_map_is_equal_to_main_chain_data(self, api: BlockchainApi, observe_governed_map_initialization):
        logging.info(f"Observed map initialization: {observe_governed_map_initialization}")
        result = api.partner_chains_node.smart_contracts.governed_map.list()
        expected_map = result.json
        actual_map = api.get_governed_map()
        actual_map = {key: string_to_hex_bytes(value) for key, value in actual_map.items()}
        assert expected_map == actual_map


class TestObserveMapChanges:
    def test_new_data_is_observed(self, insert_data, api: BlockchainApi, random_key, random_value):
        registered_change = api.subscribe_governed_map_change(key_value=(random_key, random_value))
        logging.info(f"Registered change: {registered_change}")
        actual_value = api.get_governed_map_key(random_key)
        assert random_value == actual_value

    def test_updated_data_is_observed(
        self, insert_data, api: BlockchainApi, random_key, new_value_hex_bytes, payment_key, new_value
    ):
        api.partner_chains_node.smart_contracts.governed_map.update(random_key, new_value_hex_bytes, payment_key)
        registered_change = api.subscribe_governed_map_change(key_value=(random_key, new_value))
        logging.info(f"Registered change: {registered_change}")
        actual_value = api.get_governed_map_key(random_key)
        assert new_value == actual_value

    def test_removed_data_is_observed(self, insert_data, api: BlockchainApi, random_key, payment_key):
        api.partner_chains_node.smart_contracts.governed_map.remove(random_key, payment_key)
        registered_change = api.subscribe_governed_map_change(key_value=(random_key, None))
        logging.info(f"Registered change: {registered_change}")
        actual_value = api.get_governed_map_key(random_key)
        assert actual_value is None, f"Expected empty value for key {random_key}, got {actual_value}"


class TestReinitializeMapToEmptyAddress:
    @fixture(scope="class", autouse=True)
    def set_new_governed_map_address(self, api: BlockchainApi, policy_ids, sudo):
        _, vkey = api.cardano_cli.generate_payment_keys()
        logging.info(f"Generated new payment key: {vkey}")
        bech32_vkey = cbor_to_bech32(vkey["cborHex"], "addr_vk")
        new_address = api.cardano_cli.build_address(bech32_vkey)
        logging.info(f"Generated new address: {new_address}")
        tx = api.set_governed_map_main_chain_scripts(new_address, policy_ids["GovernedMap"], sudo)
        return tx

    @fixture(scope="class", autouse=True)
    def observe_governed_map_reinitialization(self, api: BlockchainApi, set_new_governed_map_address):
        existing_key_to_observe = next(iter(api.get_governed_map()))
        change = api.subscribe_governed_map_change(key=existing_key_to_observe)
        logging.info(f"Registered change: {change}")
        return change

    def test_set_new_governed_map_address(self, set_new_governed_map_address):
        tx = set_new_governed_map_address
        assert tx._receipt.is_success, f"Failed to set new governed map address: {tx._receipt.error_message}"

    def test_governed_map_reinitialization(self, observe_governed_map_reinitialization):
        change = observe_governed_map_reinitialization
        assert not change[1], f"Value mismatch: expected empty, got {change[1]}"

    def test_observed_map_is_empty_after_changing_address(self, api: BlockchainApi):
        observed_map = api.get_governed_map()
        assert {} == observed_map, "Observed map is not empty after changing address"
