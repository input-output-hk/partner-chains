from pytest import mark
from src.blockchain_api import BlockchainApi, Wallet
from config.api_config import ApiConfig
import logging as logger

COMMITTEE_REPETITIONS_IN_PC_EPOCH = 2


@mark.xdist_group("faucet_tx")
def test_block_producer_can_update_their_metadata(genesis_utxo, api: BlockchainApi, get_wallet: Wallet, write_file):
    logger.info("Signing block producer metadata...")
    skey, vkey_hex, vkey_hash = api.cardano_cli.generate_cross_chain_keys()

    logger.info("Starting first upsert")
    metadata = {
        "url": "http://test.example",
        "hash": "0x0000000000000000000000000000000000000000000000000000000000000001",
    }
    metadata_filepath = write_file(api.partner_chains_node.run_command, metadata)

    signature = api.sign_block_producer_metadata_upsert(genesis_utxo, metadata_filepath, skey)
    assert signature.signature, "Signature is empty"
    assert signature.cross_chain_pub_key == f"0x{vkey_hex}"

    logger.info("Submitting block producer metadata...")
    tx = api.submit_block_producer_metadata_upsert(metadata, signature, wallet=get_wallet)
    assert tx.hash, "Could not submit block producer metadata"

    logger.info("Verifying block producer metadata...")

    logger.info(f"Public key: {vkey_hex}")
    storage_metadata = api.get_block_producer_metadata(vkey_hash)
    assert storage_metadata == metadata, "Block producer metadata not found in storage or incorrect"

    rpc_metadata = api.partner_chain_rpc.partner_chain_get_block_producer_metadata(vkey_hex).result
    assert rpc_metadata == metadata, "RPC did not return block producer metadata or it is incorrect"

    logger.info("Starting second upsert")
    metadata = {
        "url": "http://test.example",
        "hash": "0x0000000000000000000000000000000000000000000000000000000000000002",
    }
    metadata_filepath = write_file(api.partner_chains_node.run_command, metadata)
    signature = api.sign_block_producer_metadata_upsert(genesis_utxo, metadata_filepath, skey)
    assert signature.signature, "Signature is empty"
    assert signature.cross_chain_pub_key == f"0x{vkey_hex}"

    logger.info("Submitting block producer metadata...")
    tx = api.submit_block_producer_metadata_upsert(metadata, signature, wallet=get_wallet)
    assert tx.hash, "Could not submit block producer metadata"

    logger.info("Verifying block producer metadata...")

    logger.info(f"Public key: {vkey_hex}")
    storage_metadata = api.get_block_producer_metadata(vkey_hash)
    assert storage_metadata == metadata, "Block producer metadata not found in storage or incorrect"

    rpc_metadata = api.partner_chain_rpc.partner_chain_get_block_producer_metadata(vkey_hex).result
    assert rpc_metadata == metadata, "RPC did not return block producer metadata or it is incorrect"


@mark.xdist_group("faucet_tx")
def test_block_producer_can_delete_their_metadata(genesis_utxo, api: BlockchainApi, get_wallet: Wallet, write_file):
    logger.info("Signing block producer metadata...")
    skey, vkey_hex, vkey_hash = api.cardano_cli.generate_cross_chain_keys()


    logger.info("Starting upsert")
    metadata = {
        "url": "http://test.example",
        "hash": "0x0000000000000000000000000000000000000000000000000000000000000001",
    }
    metadata_filepath = write_file(api.partner_chains_node.run_command, metadata)

    signature = api.sign_block_producer_metadata_upsert(genesis_utxo, metadata_filepath, skey)
    assert signature.signature, "Signature is empty"
    assert signature.cross_chain_pub_key == f"0x{vkey_hex}"

    logger.info("Submitting block producer metadata...")
    tx = api.submit_block_producer_metadata_upsert(metadata, signature, wallet=get_wallet)
    assert tx.hash, "Could not submit block producer metadata"

    logger.info("Verifying block producer metadata...")

    logger.info(f"Public key: {vkey_hex}")
    storage_metadata = api.get_block_producer_metadata(vkey_hash)
    assert storage_metadata == metadata, "Block producer metadata not found in storage or incorrect"

    rpc_metadata = api.partner_chain_rpc.partner_chain_get_block_producer_metadata(vkey_hex).result
    assert rpc_metadata == metadata, "RPC did not return block producer metadata or it is incorrect"

    logger.info("Starting delete")
    signature = api.sign_block_producer_metadata_delete(genesis_utxo, skey)
    assert signature.signature, "Signature is empty"
    assert signature.cross_chain_pub_key == f"0x{vkey_hex}"

    logger.info("Submitting block producer metadata...")
    tx = api.submit_block_producer_metadata_delete(signature, wallet=get_wallet)
    assert tx.hash, "Could not submit block producer metadata"

    logger.info("Verifying block producer metadata...")

    logger.info(f"Public key: {vkey_hex}")
    storage_metadata = api.get_block_producer_metadata(vkey_hash)
    assert storage_metadata == None, "Block producer metadata not deleted from storage"

    rpc_metadata = api.partner_chain_rpc.partner_chain_get_block_producer_metadata(vkey_hex).result
    assert rpc_metadata == None, "RPC returned block producer metadata after deletion"


@mark.skip_on_new_chain
@mark.test_key('ETCM-7020')
def test_block_authors_match_committee_seats(
    api: BlockchainApi, get_pc_epoch_committee, pc_epoch, get_block_authorship_keys_dict, get_pc_epoch_blocks
):
    """
    Verifies that a pc epoch's block authors match the committee attendance.
    The committee produces blocks in a round robin manner.
    The pc epoch comes from the argument given at runtime.
    """
    logger.info(f"Verifying block authors for pc epoch {pc_epoch}...")
    block_range = get_pc_epoch_blocks(pc_epoch)["range"]
    logger.info(f"Blocks produced in epoch {pc_epoch}: {block_range}")

    # Session committee is rotated on the first block of the epoch, so we need to offset the range by 1
    block_range_with_offset = range(block_range.start + 1, block_range.stop + 1)
    logger.info(f"Blocks produced in epoch {pc_epoch} with committee offset: {block_range_with_offset}")

    committee = get_pc_epoch_committee(pc_epoch)
    committee_block_auth_pub_keys = []
    for member in committee:
        committee_block_auth_pub_keys.append(get_block_authorship_keys_dict[member["sidechainPubKey"]])

    validator_set = api.get_validator_set(get_pc_epoch_blocks(pc_epoch)[block_range_with_offset.start])
    block_authors = []
    block_slots = []
    for block_no in get_pc_epoch_blocks(pc_epoch)["range"]:
        block_author, block_slot = api.get_block_author_and_slot(
            block=get_pc_epoch_blocks(pc_epoch)[block_no], validator_set=validator_set
        )
        assert block_author, f"Could not get author of block {block_no}."
        assert (
            block_author in committee_block_auth_pub_keys
        ), f"Block {block_no} was authored by non-committee member {block_author}"
        block_authors.append(block_author)
        block_slots.append(block_slot)

    expected_block_authors = []
    for slot in block_slots:
        rank_validator = slot % len(committee)
        expected_author = committee_block_auth_pub_keys[rank_validator]
        expected_block_authors.append(expected_author)

    assert (
        expected_block_authors == block_authors
    ), f"Block authors do not match committee members for pc epoch {pc_epoch}"


@mark.test_key('ETCM-7481')
def test_block_headers_have_mc_hash(api: BlockchainApi, config: ApiConfig, pc_epoch, get_pc_epoch_blocks):
    """Test block headers have mainchain hash
    * Get blocks for current partner chain epoch
    * For each block - get the mainchain hash from the block header and timestamp
    * Check if the mainchain hash is not None
    * Get the mainchain block by hash
    * Get the latest mainchain block by timestamp
    * Check that difference between the latest and stable mainchain blocks is greater than security parameter + margin
    """
    pc_block_data = {}

    for block_no in get_pc_epoch_blocks(pc_epoch)["range"]:
        mc_block_hash = api.get_mc_hash_from_pc_block_header(get_pc_epoch_blocks(pc_epoch)[block_no])
        assert mc_block_hash, f"Could not find mainchain hash in block header for block {block_no}"
        pc_block_timestamp = api.extract_block_extrinsic_value("Timestamp", get_pc_epoch_blocks(pc_epoch)[block_no])
        pc_block_data[block_no] = (mc_block_hash, pc_block_timestamp)

    for block_no, hash_timestamp_pair in pc_block_data.items():
        hash, timestamp = hash_timestamp_pair
        stable_mc_block = api.get_mc_block_by_block_hash(hash)
        assert stable_mc_block, f"Could not find block with hash {hash} on mainchain for PC block {block_no}"

        latest_mc_block = api.get_mc_block_by_timestamp(int(timestamp / 1000))
        assert latest_mc_block, f"Could not find block with timestamp {timestamp} on mainchain for PC block {block_no}"

        latest_stable_block_diff = latest_mc_block.block_no - stable_mc_block.block_no

        logger.debug(f"Difference between latest and stable mc block: {latest_stable_block_diff} for block {block_no}")

        OFFSET = 1

        assert (
            latest_stable_block_diff + OFFSET
            >= config.main_chain.security_param + config.main_chain.block_stability_margin
        ), f"Unexpected stable block number saved in header of block {block_no}"

        if latest_stable_block_diff < config.main_chain.security_param + config.main_chain.block_stability_margin:
            logger.warning(f"Unexpected (but within offset) stable block number saved in header of block {block_no}")
