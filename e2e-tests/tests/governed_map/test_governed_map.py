from pytest import fixture, mark
from src.blockchain_api import BlockchainApi
from config.api_config import ApiConfig
import random
import string

pytestmark = [mark.governed_map]


def string_to_hex_bytes(value):
    byte_data = value.encode("utf-8")
    hex_data = f"0x{byte_data.hex()}"
    return hex_data


def hex_bytes_to_string(hex_data):
    byte_data = bytes.fromhex(hex_data[2:])
    value = byte_data.decode("utf-8")
    return value


@fixture(scope="module")
def payment_key(config: ApiConfig):
    return config.nodes_config.governance_authority.mainchain_key


@fixture(scope="class")
def random_key():
    return ''.join(random.choices(string.ascii_letters + string.digits, k=10))


@fixture(scope="class")
def random_value():
    return ''.join(random.choices(string.ascii_letters + string.digits, k=30))


@fixture(scope="class")
def insert_data(api: BlockchainApi, random_key, random_value, payment_key):
    hex_data = string_to_hex_bytes(random_value)
    result = api.partner_chains_node.smart_contracts.governed_map.insert(random_key, hex_data, payment_key)
    return result


class TestGetGovernedMap:
    def test_insert_returncode(self, insert_data):
        assert insert_data.returncode == 0

    @mark.usefixtures("insert_data")
    def test_get_returncode(self, api: BlockchainApi, random_key):
        result = api.partner_chains_node.smart_contracts.governed_map.get(random_key)
        assert result.returncode == 0

    @mark.usefixtures("insert_data")
    def test_get_value(self, api: BlockchainApi, random_key, random_value):
        result = api.partner_chains_node.smart_contracts.governed_map.get(random_key)
        value = hex_bytes_to_string(result.json)
        assert random_value == value, "Data mismatch in governed map retrieval"

    def test_get_non_existent_key(self, api: BlockchainApi):
        result = api.partner_chains_node.smart_contracts.governed_map.get("non_existent_key")
        assert {} == result.json
        assert 0 == result.returncode

    @mark.usefixtures("insert_data")
    def test_list_whole_map(self, api: BlockchainApi, random_key, random_value):
        result = api.partner_chains_node.smart_contracts.governed_map.list()
        expected_value = string_to_hex_bytes(random_value)
        assert result.returncode == 0
        assert any(
            item["key"] == random_key for item in result.json
        ), f"Key {random_key} not found in governed map list"
        found_item = next((item for item in result.json if item["key"] == random_key), None)
        assert found_item["value"] == expected_value, f"Value mismatch for key {random_key} in governed map list"


@fixture(scope="class")
def new_value():
    return string_to_hex_bytes("new_value")


class TestUpdateGovernedMap:
    @fixture(scope="class", autouse=True)
    def update_data(self, api: BlockchainApi, insert_data, random_key, new_value, payment_key):
        result = api.partner_chains_node.smart_contracts.governed_map.update(random_key, new_value, payment_key)
        return result

    def test_update_returncode(self, update_data):
        assert update_data.returncode == 0

    def test_update_value(self, api: BlockchainApi, random_key, new_value):
        get_result = api.partner_chains_node.smart_contracts.governed_map.get(random_key)
        assert new_value == get_result.json, "Data mismatch after update in governed map retrieval"


class TestUpdateGovernedMapWithExpectedCurrentValue:
    @fixture(scope="class", autouse=True)
    def update_data(self, api: BlockchainApi, insert_data, random_key, random_value, new_value, payment_key):
        result = api.partner_chains_node.smart_contracts.governed_map.update(
            random_key, new_value, payment_key, current_value=string_to_hex_bytes(random_value)
        )
        return result

    def test_update_returncode(self, update_data):
        assert update_data.returncode == 0

    def test_update_value(self, api: BlockchainApi, random_key, new_value):
        get_result = api.partner_chains_node.smart_contracts.governed_map.get(random_key)
        assert new_value == get_result.json, "Data mismatch after update in governed map retrieval"


class TestUpdateGovernedMapWithNonMatchingCurrentValue:
    @fixture(scope="class", autouse=True)
    def update_data(self, api: BlockchainApi, insert_data, random_key, new_value, payment_key):
        result = api.partner_chains_node.smart_contracts.governed_map.update(
            random_key, new_value, payment_key, current_value=string_to_hex_bytes("non_matching_value")
        )
        return result

    def test_update_returncode(self, update_data):
        assert update_data.returncode == 1

    def test_update_value(self, api: BlockchainApi, random_key, random_value):
        get_result = api.partner_chains_node.smart_contracts.governed_map.get(random_key)
        assert (
            string_to_hex_bytes(random_value) == get_result.json
        ), "Data should not be updated in governed map retrieval"
