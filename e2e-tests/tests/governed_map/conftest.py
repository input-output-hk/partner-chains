from pytest import fixture, mark
from src.blockchain_api import BlockchainApi
from config.api_config import ApiConfig
import logging
import random
import string


def pytest_collection_modifyitems(items):
    for item in items:
        if "tests/governed_map" in item.nodeid:
            item.add_marker(mark.governed_map)
        if "observability" in item.nodeid:
            item.add_marker(
                mark.skipif(
                    item.config.getoption("--env") != "local",
                    reason="Governed Map is not observable in reasonable time on environment other than local",
                )
            )


def string_to_hex_bytes(value):
    byte_data = value.encode("utf-8")
    hex_data = f"0x{byte_data.hex()}"
    return hex_data


def hex_bytes_to_string(hex_data):
    byte_data = bytes.fromhex(hex_data[2:])
    value = byte_data.decode("utf-8")
    return value


def random_string(length=10):
    return ''.join(random.choices(string.ascii_letters + string.digits, k=length))


@fixture(scope="session")
def payment_key(config: ApiConfig, governance_skey_with_cli):
    return config.nodes_config.governance_authority.mainchain_key


@fixture(scope="class")
def random_key():
    key = f"GovMap_{random_string(10)}"
    logging.info(f"Generated random key for Governed Map: {key}")
    return key


@fixture(scope="class")
def random_value():
    value = f"GovMap_{random_string(30)}"
    logging.info(f"Generated random value for Governed Map: {value}")
    return value


@fixture(scope="class")
def new_value():
    value = f"GovMapUpdate_{random_string(30)}"
    logging.info(f"Generated new random value for Governed Map: {value}")
    return value


@fixture(scope="class")
def new_value_hex_bytes(new_value):
    return string_to_hex_bytes(new_value)


@fixture(scope="class")
def insert_data(api: BlockchainApi, random_key, random_value, payment_key):
    logging.info(f"Inserting data into Governed Map with key: {random_key} and value: {random_value}")
    hex_data = string_to_hex_bytes(random_value)
    result = api.partner_chains_node.smart_contracts.governed_map.insert(random_key, hex_data, payment_key)
    yield result
    api.partner_chains_node.smart_contracts.governed_map.remove(random_key, payment_key)
    logging.info(f"Cleaned up test data from Governed Map with key: {random_key}")
