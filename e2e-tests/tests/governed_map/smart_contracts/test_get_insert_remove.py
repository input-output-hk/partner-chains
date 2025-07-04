from pytest import fixture, mark
from src.blockchain_api import BlockchainApi
from tests.governed_map.conftest import string_to_hex_bytes, hex_bytes_to_string

pytestmark = [mark.xdist_group(name="governance_action")]


class TestGet:
    def test_insert_returncode(self, insert_data):
        assert 0 == insert_data.returncode

    @mark.usefixtures("insert_data")
    def test_get_returncode(self, api: BlockchainApi, genesis_utxo, random_key):
        result = api.partner_chains_node.smart_contracts.governed_map.get(genesis_utxo, random_key)
        assert 0 == result.returncode

    @mark.usefixtures("insert_data")
    def test_get_value(self, api: BlockchainApi, genesis_utxo, random_key, random_value):
        result = api.partner_chains_node.smart_contracts.governed_map.get(genesis_utxo, random_key)
        value = hex_bytes_to_string(result.json)
        assert random_value == value, "Data mismatch in governed map retrieval"

    def test_get_non_existent_key(self, api: BlockchainApi, genesis_utxo):
        result = api.partner_chains_node.smart_contracts.governed_map.get(genesis_utxo, "non_existent_key")
        assert {} == result.json
        assert 0 == result.returncode

    @mark.usefixtures("insert_data")
    def test_list_whole_map(self, api: BlockchainApi, genesis_utxo, random_key, random_value):
        result = api.partner_chains_node.smart_contracts.governed_map.list(genesis_utxo)
        expected_value = string_to_hex_bytes(random_value)
        assert 0 == result.returncode
        assert random_key in result.json
        assert expected_value == result.json[random_key], f"Value mismatch for key {random_key} in governed map list"


class TestInsertTwice:
    @fixture(scope="class")
    def insert_twice_with_the_same_value(self, static_api: BlockchainApi, insert_data, genesis_utxo, random_key, random_value, payment_key):
        hex_data = string_to_hex_bytes(random_value)
        result = static_api.partner_chains_node.smart_contracts.governed_map.insert(genesis_utxo, random_key, hex_data, payment_key)
        return result

    @fixture(scope="class")
    def insert_twice_with_different_value(
        self, static_api: BlockchainApi, insert_data, genesis_utxo, random_key, new_value_hex_bytes, payment_key
    ):
        hex_data = string_to_hex_bytes(new_value_hex_bytes)
        result = static_api.partner_chains_node.smart_contracts.governed_map.insert(genesis_utxo, random_key, hex_data, payment_key)
        return result

    def test_insert_with_the_same_value(self, insert_twice_with_the_same_value):
        result = insert_twice_with_the_same_value
        assert 0 == result.returncode
        assert {} == result.json

    def test_value_remains_the_same(
        self, api: BlockchainApi, insert_twice_with_the_same_value, genesis_utxo, random_key, random_value
    ):
        get_result = api.partner_chains_node.smart_contracts.governed_map.get(genesis_utxo, random_key)
        value = hex_bytes_to_string(get_result.json)
        assert random_value == value

    def test_insert_with_different_value(self, insert_twice_with_different_value):
        result = insert_twice_with_different_value
        assert 1 == result.returncode
        assert "There is already a value stored for key" in result.stderr

    def test_value_was_not_updated(
        self, api: BlockchainApi, insert_twice_with_different_value, genesis_utxo, random_key, random_value
    ):
        get_result = api.partner_chains_node.smart_contracts.governed_map.get(genesis_utxo, random_key)
        value = hex_bytes_to_string(get_result.json)
        assert 0 == get_result.returncode
        assert random_value == value


class TestRemove:
    @fixture(scope="class")
    def remove_data(self, static_api: BlockchainApi, insert_data, genesis_utxo, random_key, payment_key):
        result = static_api.partner_chains_node.smart_contracts.governed_map.remove(genesis_utxo, random_key, payment_key)
        return result

    def test_remove_returncode(self, remove_data):
        assert 0 == remove_data.returncode

    def test_get_after_remove(self, api: BlockchainApi, remove_data, genesis_utxo, random_key):
        result = api.partner_chains_node.smart_contracts.governed_map.get(genesis_utxo, random_key)
        assert {} == result.json
        assert 0 == result.returncode

    def test_remove_non_existent_key(self, api: BlockchainApi, genesis_utxo, payment_key):
        result = api.partner_chains_node.smart_contracts.governed_map.remove(genesis_utxo, "non_existent_key", payment_key)
        assert 0 == result.returncode
        assert {} == result.json
        assert "There is no value stored for key 'non_existent_key'. Skipping remove." in result.stderr
