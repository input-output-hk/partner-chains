from pytest import fixture
from src.blockchain_api import BlockchainApi
from src.cardano_cli import cbor_to_bech32
from conftest import string_to_hex_bytes
import logging


class TestObserveMap:
    @fixture(scope="session")
    def sudo(self, api: BlockchainApi, secrets):
        sudo_config = secrets["wallets"]["sudo"]
        sudo = api.get_wallet(
            address=sudo_config["address"],
            public_key=sudo_config["public_key"],
            secret=sudo_config["secret_seed"],
            scheme=sudo_config["scheme"],
        )
        return sudo

    @fixture(scope="class")
    def set_validator_address(self, api: BlockchainApi, addresses, policy_ids, sudo):
        tx = api.set_governed_map_address(addresses["GovernedMapValidator"], policy_ids["GovernedMap"], sudo)
        return tx

    def test_set_main_chain_scripts(self, set_validator_address):
        tx = set_validator_address
        assert tx._receipt.is_success, f"Failed to set new governed map address: {tx._receipt.error_message}"

    def test_observed_map_is_equal_to_main_chain_data(self, api: BlockchainApi, random_key):
        result = api.partner_chains_node.smart_contracts.governed_map.list()
        expected_map = result.json
        actual_map = api.get_governed_map()
        actual_map = {key: string_to_hex_bytes(value) for key, value in actual_map.items()}
        assert expected_map == actual_map

    def test_new_data_is_observed(self, insert_data, api: BlockchainApi, random_key, random_value):
        registered_change = api.subscribe_governed_map_change(key=random_key)
        logging.info(f"Registered change: {registered_change}")
        actual_value = api.get_governed_map_key(random_key)
        assert random_value == actual_value

    def test_updated_data_is_observed(
        self, insert_data, api: BlockchainApi, random_key, new_value_hex_bytes, payment_key, new_value
    ):
        api.partner_chains_node.smart_contracts.governed_map.update(random_key, new_value_hex_bytes, payment_key)
        registered_change = api.subscribe_governed_map_change(key=random_key, value=new_value)
        logging.info(f"Registered change: {registered_change}")
        actual_value = api.get_governed_map_key(random_key)
        assert new_value == actual_value

    def test_set_new_governed_map_address(self, api: BlockchainApi, policy_ids, sudo):
        _, vkey = api.cardano_cli.generate_payment_keys()
        logging.info(f"Generated new payment key: {vkey}")
        bech32_vkey = cbor_to_bech32(vkey["cborHex"], "addr_vk")
        new_address = api.cardano_cli.build_address(bech32_vkey)
        logging.info(f"Generated new address: {new_address}")
        api.set_governed_map_address(new_address, policy_ids["GovernedMap"], sudo)

    def test_observed_map_is_empty_after_changing_address(self, api: BlockchainApi, policy_ids, sudo):
        existing_key_to_observe = next(iter(api.get_governed_map()))
        _, vkey = api.cardano_cli.generate_payment_keys()
        logging.info(f"Generated new payment key: {vkey}")
        bech32_vkey = cbor_to_bech32(vkey["cborHex"], "addr_vk")
        new_address = api.cardano_cli.build_address(bech32_vkey)
        logging.info(f"Generated new address: {new_address}")
        api.set_governed_map_address(new_address, policy_ids["GovernedMap"], sudo)
        change = api.subscribe_governed_map_change(key=existing_key_to_observe)
        logging.info(f"Registered change: {change}")
        assert not change[1], f"Value mismatch: expected empty, got {change[1]}"
        observed_map = api.get_governed_map()
        assert {} == observed_map, "Observed map is not empty after changing address"
