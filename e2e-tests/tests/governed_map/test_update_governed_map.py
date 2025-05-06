from pytest import fixture
from src.blockchain_api import BlockchainApi
from conftest import string_to_hex_bytes


class TestUpdate:
    @fixture(scope="class", autouse=True)
    def update_data(self, api: BlockchainApi, insert_data, random_key, new_value_hex_bytes, payment_key):
        result = api.partner_chains_node.smart_contracts.governed_map.update(
            random_key, new_value_hex_bytes, payment_key
        )
        return result

    def test_update_returncode(self, update_data):
        assert update_data.returncode == 0

    def test_update_value(self, api: BlockchainApi, random_key, new_value_hex_bytes):
        get_result = api.partner_chains_node.smart_contracts.governed_map.get(random_key)
        assert new_value_hex_bytes == get_result.json, "Data mismatch after update in governed map retrieval"


class TestUpdateWithTheSameValue:
    @fixture(scope="class", autouse=True)
    def update_data(self, api: BlockchainApi, insert_data, random_key, random_value, payment_key):
        result = api.partner_chains_node.smart_contracts.governed_map.update(
            random_key, string_to_hex_bytes(random_value), payment_key
        )
        return result

    def test_update_response(self, update_data):
        assert update_data.returncode == 0
        assert update_data.json == {}

    def test_value_remains_the_same(self, api: BlockchainApi, random_key, random_value):
        get_result = api.partner_chains_node.smart_contracts.governed_map.get(random_key)
        assert string_to_hex_bytes(random_value) == get_result.json


class TestUpdateWithExpectedCurrentValue:
    @fixture(scope="class", autouse=True)
    def update_data(self, api: BlockchainApi, insert_data, random_key, random_value, new_value_hex_bytes, payment_key):
        result = api.partner_chains_node.smart_contracts.governed_map.update(
            random_key, new_value_hex_bytes, payment_key, current_value=string_to_hex_bytes(random_value)
        )
        return result

    def test_update_returncode(self, update_data):
        assert update_data.returncode == 0

    def test_update_value(self, api: BlockchainApi, random_key, new_value_hex_bytes):
        get_result = api.partner_chains_node.smart_contracts.governed_map.get(random_key)
        assert new_value_hex_bytes == get_result.json, "Data mismatch after update in governed map retrieval"


class TestUpdateWithExpectedCurrentValueAndTheSameValue:
    @fixture(scope="class", autouse=True)
    def update_data(self, api: BlockchainApi, insert_data, random_key, random_value, payment_key):
        result = api.partner_chains_node.smart_contracts.governed_map.update(
            random_key, string_to_hex_bytes(random_value), payment_key, current_value=string_to_hex_bytes(random_value)
        )
        return result

    def test_update_response(self, update_data):
        assert update_data.returncode == 0
        assert update_data.json == {}

    def test_value_remains_the_same(self, api: BlockchainApi, random_key, random_value):
        get_result = api.partner_chains_node.smart_contracts.governed_map.get(random_key)
        assert string_to_hex_bytes(random_value) == get_result.json


class TestUpdateWithNonMatchingCurrentValue:
    @fixture(scope="class", autouse=True)
    def update_data(self, api: BlockchainApi, insert_data, random_key, new_value_hex_bytes, payment_key):
        result = api.partner_chains_node.smart_contracts.governed_map.update(
            random_key, new_value_hex_bytes, payment_key, current_value=string_to_hex_bytes("non_matching_value")
        )
        return result

    def test_update_returncode_and_message(self, update_data, random_key):
        assert update_data.returncode == 1
        assert f"Value for key '{random_key}' is set to a different value than expected" in update_data.stderr

    def test_update_value(self, api: BlockchainApi, random_key, random_value):
        get_result = api.partner_chains_node.smart_contracts.governed_map.get(random_key)
        assert (
            string_to_hex_bytes(random_value) == get_result.json
        ), "Data should not be updated in governed map retrieval"


class TestUpdateWithNonExistentKey:
    @fixture(scope="class", autouse=True)
    def update_data(self, api: BlockchainApi, random_key, new_value_hex_bytes, payment_key):
        result = api.partner_chains_node.smart_contracts.governed_map.update(
            random_key, new_value_hex_bytes, payment_key
        )
        return result

    def test_update_returncode(self, update_data):
        assert update_data.returncode == 1

    def test_update_value(self, api: BlockchainApi, random_key):
        get_result = api.partner_chains_node.smart_contracts.governed_map.get(random_key)
        assert {} == get_result.json
