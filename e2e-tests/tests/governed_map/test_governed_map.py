from pytest import fixture, mark
from src.blockchain_api import BlockchainApi
from config.api_config import ApiConfig
import logging
import random
import string

pytestmark = [mark.governed_map]


@fixture(scope="module")
def payment_key(config: ApiConfig):
    return config.nodes_config.governance_authority.mainchain_key


@fixture(scope="module")
def random_key():
    return ''.join(random.choices(string.ascii_letters + string.digits, k=10))


@fixture(scope="module")
def random_value():
    return ''.join(random.choices(string.ascii_letters + string.digits, k=30))


@fixture(scope="module")
def insert_data(api: BlockchainApi, random_key, random_value, payment_key):
    byte_data = random_value.encode("utf-8")
    hex_data = byte_data.hex()
    result = api.partner_chains_node.smart_contracts.governed_map.insert(random_key, hex_data, payment_key)
    return result


def test_insert_into_governed_map(insert_data):
    assert insert_data.returncode == 0


@mark.usefixtures("insert_data")
def test_get_returncode(api: BlockchainApi, random_key):
    result = api.partner_chains_node.smart_contracts.governed_map.get(random_key)
    assert result.returncode == 0


@mark.usefixtures("insert_data")
def test_get_value(api: BlockchainApi, random_key, random_value):
    result = api.partner_chains_node.smart_contracts.governed_map.get(random_key)
    hex_data = result.json
    byte_data = bytes.fromhex(hex_data[2:])
    value = byte_data.decode("utf-8")
    assert random_value == value, "Data mismatch in governed map retrieval"


def test_get_non_existent_key(api: BlockchainApi):
    result = api.partner_chains_node.smart_contracts.governed_map.get("non_existent_key")
    assert {} == result.json
    assert 0 == result.returncode
