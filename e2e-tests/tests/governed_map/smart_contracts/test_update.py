from pytest import fixture, mark
from src.blockchain_api import BlockchainApi
from tests.governed_map.conftest import string_to_hex_bytes

pytestmark = [mark.xdist_group(name="governance_action")]


@mark.staging
class TestUpdate:
    @fixture(scope="class", autouse=True)
    def update_data(self, api: BlockchainApi, insert_data, genesis_utxo, random_key, new_value_hex_bytes, payment_key):
        result = api.partner_chains_node.smart_contracts.governed_map.update(
            genesis_utxo,
            random_key,
            new_value_hex_bytes,
            payment_key
        )
        return result

    @mark.test_key("ETCM-10369")
    def test_update_returncode(self, update_data):
        assert update_data.returncode == 0

    @mark.test_key("ETCM-10370")
    def test_update_value(self, api: BlockchainApi, genesis_utxo, random_key, new_value_hex_bytes):
        get_result = api.partner_chains_node.smart_contracts.governed_map.get(genesis_utxo, random_key)
        assert new_value_hex_bytes == get_result.json, "Data mismatch after update in governed map retrieval"


@mark.staging
class TestUpdateWithTheSameValue:
    @fixture(scope="class", autouse=True)
    def update_data(self, api: BlockchainApi, insert_data, genesis_utxo, random_key, random_value, payment_key):
        result = api.partner_chains_node.smart_contracts.governed_map.update(
            genesis_utxo,
            random_key,
            string_to_hex_bytes(random_value),
            payment_key
        )
        return result

    @mark.test_key("ETCM-10371")
    def test_update_response(self, update_data):
        assert update_data.returncode == 0
        assert update_data.json == {}

    @mark.test_key("ETCM-10372")
    def test_value_remains_the_same(self, api: BlockchainApi, genesis_utxo, random_key, random_value):
        get_result = api.partner_chains_node.smart_contracts.governed_map.get(genesis_utxo, random_key)
        assert string_to_hex_bytes(random_value) == get_result.json


@mark.staging
class TestUpdateWithExpectedCurrentValue:
    @fixture(scope="class", autouse=True)
    def update_data(self, api: BlockchainApi, insert_data, genesis_utxo, random_key, random_value, new_value_hex_bytes, payment_key):
        result = api.partner_chains_node.smart_contracts.governed_map.update(
            genesis_utxo,
            random_key,
            new_value_hex_bytes,
            payment_key,
            current_value=string_to_hex_bytes(random_value)
        )
        return result

    @mark.test_key("ETCM-10373")
    def test_update_returncode(self, update_data):
        assert update_data.returncode == 0

    @mark.test_key("ETCM-10374")
    def test_update_value(self, api: BlockchainApi, genesis_utxo, random_key, new_value_hex_bytes):
        get_result = api.partner_chains_node.smart_contracts.governed_map.get(genesis_utxo, random_key)
        assert new_value_hex_bytes == get_result.json, "Data mismatch after update in governed map retrieval"


@mark.staging
class TestUpdateWithExpectedCurrentValueAndTheSameValue:
    @fixture(scope="class", autouse=True)
    def update_data(self, api: BlockchainApi, insert_data, genesis_utxo, random_key, random_value, payment_key):
        result = api.partner_chains_node.smart_contracts.governed_map.update(
            genesis_utxo,
            random_key,
            string_to_hex_bytes(random_value),
            payment_key,
            current_value=string_to_hex_bytes(random_value)
        )
        return result

    @mark.test_key("ETCM-10375")
    def test_update_response(self, update_data):
        assert update_data.returncode == 0
        assert update_data.json == {}

    @mark.test_key("ETCM-10376")
    def test_value_remains_the_same(self, api: BlockchainApi, genesis_utxo, random_key, random_value):
        get_result = api.partner_chains_node.smart_contracts.governed_map.get(genesis_utxo, random_key)
        assert string_to_hex_bytes(random_value) == get_result.json


@mark.staging
class TestUpdateWithNonMatchingCurrentValue:
    @fixture(scope="class", autouse=True)
    def update_data(self, api: BlockchainApi, insert_data, genesis_utxo, random_key, new_value_hex_bytes, payment_key):
        result = api.partner_chains_node.smart_contracts.governed_map.update(
            genesis_utxo,
            random_key,
            new_value_hex_bytes,
            payment_key,
            current_value=string_to_hex_bytes("non_matching_value")
        )
        return result

    @mark.test_key("ETCM-10377")
    def test_update_returncode_and_message(self, update_data, random_key):
        assert update_data.returncode == 1
        assert f"Value for key '{random_key}' is set to a different value than expected" in update_data.stderr

    @mark.test_key("ETCM-10378")
    def test_update_value(self, api: BlockchainApi, genesis_utxo, random_key, random_value):
        get_result = api.partner_chains_node.smart_contracts.governed_map.get(genesis_utxo, random_key)
        assert (
            string_to_hex_bytes(random_value) == get_result.json
        ), "Data should not be updated in governed map retrieval"


@mark.staging
class TestUpdateWithNonExistentKey:
    @fixture(scope="class", autouse=True)
    def update_data(self, api: BlockchainApi, genesis_utxo, random_key, new_value_hex_bytes, payment_key):
        result = api.partner_chains_node.smart_contracts.governed_map.update(
            genesis_utxo,
            random_key,
            new_value_hex_bytes,
            payment_key
        )
        return result

    @mark.test_key("ETCM-10379")
    def test_update_returncode(self, update_data):
        assert update_data.returncode == 1

    @mark.test_key("ETCM-10380")
    def test_update_value(self, api: BlockchainApi, genesis_utxo, random_key):
        get_result = api.partner_chains_node.smart_contracts.governed_map.get(genesis_utxo, random_key)
        assert {} == get_result.json
