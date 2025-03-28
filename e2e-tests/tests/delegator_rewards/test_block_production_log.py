from pytest import mark
from src.blockchain_api import BlockchainApi
from config.api_config import ApiConfig
import logging


@mark.block_production_log
def test_block_production_log_pallet(api: BlockchainApi, config: ApiConfig):
    block = api.get_block()
    block_no = block["header"]["number"]
    block_hash = block["header"]["hash"]
    block_production_log = api.get_block_production_log(block_hash=block_hash)
    assert block_production_log
    block_range = range(block_no - len(block_production_log), block_no)
    logging.info(
        f"Verifying block authors for slots {block_production_log[0][0]}..{block_production_log[-1][0]} "
        f"(blocks {block_range})"
    )
    block_production_log.reverse()
    for slot, block_producer_id in block_production_log:
        committee = api.get_validator_set(block).value
        author_index = slot % len(committee)
        expected_node = next(
            x for x in config.nodes_config.nodes.values() if x.aura_public_key == committee[author_index][1]["aura"]
        )
        if "Incentivized" in block_producer_id:
            cross_chain_public, stake_pool_public_key = block_producer_id["Incentivized"]
            assert (
                cross_chain_public == expected_node.public_key
            ), f"Incorrect author: block {block_no} ({block_hash}) was authored by non-committee member"
            expected_spo_key = api.read_cardano_key_file(expected_node["keys_files"]["spo_public_key"])
            assert (
                stake_pool_public_key[2:] == expected_spo_key
            ), f"Incorrect SPO: block {block_no} ({block_hash}) author has incorrect SPO"
        elif "ProBono" in block_producer_id:
            cross_chain_public = block_producer_id["ProBono"]
            assert (
                cross_chain_public == expected_node.public_key
            ), f"Incorrect author: block {block_no} ({block_hash}) was authored by non-committee member"
        else:
            assert False, f"Invalid block producer id: {block_producer_id}"
        block_no = block_no - 1
        block = api.get_block(block_no)
        block_hash = block["header"]["hash"]
