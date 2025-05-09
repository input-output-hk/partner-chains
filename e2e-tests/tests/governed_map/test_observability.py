from pytest import fixture, mark
from config.api_config import ApiConfig
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
def set_governed_map_scripts(api: BlockchainApi, addresses, policy_ids, sudo):
    tx = api.set_governed_map_main_chain_scripts(addresses["GovernedMapValidator"], policy_ids["GovernedMap"], sudo)
    return tx


@fixture(scope="session", autouse=True)
def observe_governed_map_initialization(api: BlockchainApi, set_governed_map_scripts):
    result = api.subscribe_governed_map_initialization()
    return result


class TestInitializeMap:
    def test_set_main_chain_scripts(self, set_governed_map_scripts):
        tx = set_governed_map_scripts
        assert tx._receipt.is_success, f"Failed to set new governed map address: {tx._receipt.error_message}"

    def test_governed_map_initialization(self, observe_governed_map_initialization):
        logging.info(f"Governed Map initialized: {observe_governed_map_initialization}")
        assert observe_governed_map_initialization

    def test_map_is_equal_to_main_chain_data(self, api: BlockchainApi, wait_until, config: ApiConfig):
        current_mc_block = api.get_mc_block()
        result = api.partner_chains_node.smart_contracts.governed_map.list()
        # wait for any changes in the map to become observable, i.e. teardown phase from smart-contracts tests
        wait_until(lambda: api.get_mc_block() > current_mc_block + config.main_chain.security_param)
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


class TestReinitializeMap:
    @fixture(scope="class")
    def insert_data_and_wait_until_observed(self, insert_data, api: BlockchainApi, random_key, random_value):
        api.subscribe_governed_map_change(key_value=(random_key, random_value))
        return api.get_governed_map()

    @fixture(scope="session")
    def create_new_governed_map_address(self, api: BlockchainApi):
        _, vkey = api.cardano_cli.generate_payment_keys()
        logging.info(f"Generated new payment key: {vkey}")
        bech32_vkey = cbor_to_bech32(vkey["cborHex"], "addr_vk")
        new_address = api.cardano_cli.build_address(bech32_vkey)
        logging.info(f"New address for Governed Map: {new_address}")
        return new_address

    @fixture(scope="class", autouse=True)
    def set_new_governed_map_scripts(
        self, create_new_governed_map_address, insert_data_and_wait_until_observed, api: BlockchainApi, policy_ids, sudo
    ):
        new_address = create_new_governed_map_address
        tx = api.set_governed_map_main_chain_scripts(new_address, policy_ids["GovernedMap"], sudo)
        return tx

    @fixture(scope="class", autouse=True)
    def observe_governed_map_reinitialization(self, api: BlockchainApi, set_new_governed_map_scripts):
        result = api.subscribe_governed_map_initialization()
        return result

    def test_set_new_governed_map_address(self, set_new_governed_map_scripts):
        tx = set_new_governed_map_scripts
        assert tx._receipt.is_success, f"Failed to set new governed map address: {tx._receipt.error_message}"

    def test_governed_map_was_reinitialized(self, observe_governed_map_reinitialization):
        logging.info(f"Governed Map reinitialized: {observe_governed_map_reinitialization}")
        assert observe_governed_map_reinitialization

    def test_observed_map_is_empty_after_changing_address(self, api: BlockchainApi):
        observed_map = api.get_governed_map()
        assert {} == observed_map, "Observed map is not empty after changing address"

    def test_revert_map_to_previous_address(
        self, api: BlockchainApi, addresses, policy_ids, insert_data_and_wait_until_observed, sudo
    ):
        tx = api.set_governed_map_main_chain_scripts(addresses["GovernedMapValidator"], policy_ids["GovernedMap"], sudo)
        assert tx._receipt.is_success, f"Failed to revert governed map address: {tx._receipt.error_message}"

        result = api.subscribe_governed_map_initialization()
        assert result, "Failed to observe reinitialization of governed map after reverting address"

        observed_map = api.get_governed_map()
        assert observed_map == insert_data_and_wait_until_observed, "Observed map does not match the initial state"
