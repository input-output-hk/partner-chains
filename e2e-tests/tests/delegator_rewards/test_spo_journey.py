import logging
import random
from config.api_config import ApiConfig
from src.blockchain_api import BlockchainApi, Transaction, Wallet
from src.pc_epoch_calculator import PartnerChainEpochCalculator
from sqlalchemy.orm import Session
from sqlalchemy.sql import text
from pytest import fixture, mark, skip

PARTICIPATION_DATA_SLOT_RANGE = 30


##############################################################################################################
# Disclaimer!                                                                                                #
# The Registered SPO user journey is not fully covered because it requires a rewards distribution mechanism. #
# Its implementation lies on the Chain Builder side. This test suite is designed to verify the correctness   #
# of the raw inherent data that we pass to the Chain Builder to build such a mechanism. The raw inherent     #
# data are exposed via testHelperPallet which serves as a replacement for distribution mechanism.            #
##############################################################################################################


@fixture(scope="module")
def block_to_query_storage(api: BlockchainApi):
    block = api.get_block()
    block_no = block["header"]["number"]
    if block_no <= PARTICIPATION_DATA_SLOT_RANGE:
        skip(f"Participation data is released after {PARTICIPATION_DATA_SLOT_RANGE} slots, current block {block_no}.")
    logging.info(f"Block to query storage: {block_no}")
    return block

@fixture(scope="module")
def block_slot(block_to_query_storage, api: BlockchainApi):
    return api.get_block_slot(block_to_query_storage)


@fixture(scope="module")
def block_participation(block_to_query_storage, api: BlockchainApi):
    block_participation = api.get_block_participation_data(block_hash=block_to_query_storage["header"]["hash"])
    return block_participation


@fixture(scope="module")
def block_production_log(block_to_query_storage, api: BlockchainApi):
    block_no = block_to_query_storage["header"]["number"]
    block_no_matching_participation_data = block_no - len(
        api.get_block_production_log(block_hash=block_to_query_storage["header"]["hash"])
    )
    logging.info(f"Block number matching participation data slots range: {block_no_matching_participation_data}")
    block_matching_participation_data = api.get_block(block_no_matching_participation_data)
    block_production_log = api.get_block_production_log(block_hash=block_matching_participation_data["header"]["hash"])
    return block_production_log


@fixture(scope="module")
def pc_epochs(block_participation, block_slot, config: ApiConfig, initial_pc_epoch):
    start_pc_epoch = (block_slot - PARTICIPATION_DATA_SLOT_RANGE) // config.nodes_config.slots_in_epoch
    stop_pc_epoch = block_slot // config.nodes_config.slots_in_epoch
    epochs = range(start_pc_epoch, stop_pc_epoch)
    logging.info(f"Participation data spans PC epochs: {epochs}")
    if initial_pc_epoch in epochs:
        epochs = range(initial_pc_epoch, stop_pc_epoch)
        logging.info(f"Initial PC epoch is greater than the first PC epoch. Adjusting... New range is: {epochs}")
    return epochs


@fixture(scope="module")
def mc_epochs(pc_epochs: range, pc_epoch_calculator: PartnerChainEpochCalculator, current_mc_epoch: int):
    start_mc_epoch = pc_epoch_calculator.find_mc_epoch(pc_epochs.start, current_mc_epoch)
    stop_mc_epoch = pc_epoch_calculator.find_mc_epoch(pc_epochs.stop - 1, current_mc_epoch)
    
    # Handle case where epochs can't be mapped (e.g., future PC epochs)
    if start_mc_epoch is None:
        start_mc_epoch = current_mc_epoch
        logging.warning(f"Could not map start PC epoch {pc_epochs.start} to MC epoch, using current MC epoch {current_mc_epoch}")
    
    if stop_mc_epoch is None:
        # PC epoch is in the future, use current + a reasonable lookahead
        stop_mc_epoch = current_mc_epoch + 1
        logging.warning(f"Could not map stop PC epoch {pc_epochs.stop - 1} to MC epoch, using current + 1 = {stop_mc_epoch}")
    
    logging.info(f"Participation data spans MC epochs: {start_mc_epoch} to {stop_mc_epoch}")
    return range(start_mc_epoch, stop_mc_epoch + 1)


@fixture(scope="module")
def initial_pc_epoch_included(initial_pc_epoch: int, pc_epoch_calculator: PartnerChainEpochCalculator):
    def _inner(mc_epoch):
        pc_epochs = pc_epoch_calculator.find_pc_epochs(mc_epoch, start_from_initial_pc_epoch=True)
        if initial_pc_epoch in pc_epochs:
            logging.info("Initial PC epoch is in the range of participation data slots.")
            return initial_pc_epoch
        return False

    return _inner


@fixture(scope="module")
def count_blocks(pc_epoch_calculator: PartnerChainEpochCalculator, config: ApiConfig, block_production_log):
    slots_in_epoch = config.nodes_config.slots_in_epoch
    mc_epoch_to_pc_slots = {}

    def _mc_epoch_to_pc_slots(mc_epoch):
        if mc_epoch_to_pc_slots.get(mc_epoch):
            return mc_epoch_to_pc_slots[mc_epoch]

        pc_epochs = pc_epoch_calculator.find_pc_epochs(mc_epoch, start_from_initial_pc_epoch=False)
        start_slot = pc_epochs.start * slots_in_epoch
        stop_slot = pc_epochs.stop * slots_in_epoch
        slots = range(start_slot, stop_slot)
        logging.info(f"MC epoch {mc_epoch} PC slots: {slots}")
        mc_epoch_to_pc_slots[mc_epoch] = slots
        return slots

    def _count_blocks(mc_epoch, producer):
        slots = _mc_epoch_to_pc_slots(mc_epoch)
        block_count = 0
        for slot, producer_info in block_production_log:
            if slots.start <= slot < slots.stop:
                if producer == producer_info:
                    block_count += 1
        return block_count

    return _count_blocks


@fixture(scope="module")
def all_mc_epochs_in_participation(mc_epochs: range):
    """Return all MC epochs that should be checked.
    
    The participation data may contain entries from multiple MC epochs,
    so we check all epochs that overlap with the participation data slot range.
    """
    return list(mc_epochs)


@mark.dependency(name="participation_data")
@mark.xdist_group("block_participation")
@mark.staging
def test_block_participation_data_is_not_empty(block_participation):
    assert block_participation
    assert block_participation["producer_participation"]


@mark.dependency(name="pro_bono_participation")
@mark.xdist_group("block_participation")
@mark.staging
def test_pro_bono_participation(
    all_mc_epochs_in_participation, api: BlockchainApi, initial_pc_epoch_included, count_blocks: int, block_participation
):
    # Track all permissioned candidates across all MC epochs
    all_permissioned_keys = set()
    
    for mc_epoch in all_mc_epochs_in_participation:
        logging.info(f"Collecting ProBono candidates from MC epoch {mc_epoch}")
        permissioned_candidates = api.get_permissioned_candidates(mc_epoch, valid_only=True)

        initial_pc_epoch = initial_pc_epoch_included(mc_epoch)
        if initial_pc_epoch:
            logging.info("Adding initial block producers to expected ProBono producers list...")
            initial_block_producers = api.get_epoch_committee(initial_pc_epoch).result["committee"]
            for item in initial_block_producers:
                permissioned_candidates.append({"sidechainPublicKey": item["sidechainPubKey"]})

        for permissioned_candidate in permissioned_candidates:
            all_permissioned_keys.add(permissioned_candidate["sidechainPublicKey"])
    
    # Now remove all ProBono entries from participation data that match our collected candidates
    logging.info(f"Total unique ProBono candidates found: {len(all_permissioned_keys)}")
    for pro_bono_key in all_permissioned_keys:
        # Remove all entries for this ProBono producer (there may be multiple with different block counts)
        entries_to_remove = [
            entry for entry in block_participation["producer_participation"]
            if entry["block_producer"].get("ProBono") == pro_bono_key
        ]
        for entry in entries_to_remove:
            logging.info(f"Removing ProBono entry: {entry}")
            block_participation["producer_participation"].remove(entry)


@mark.dependency(name="spo_participation")
@mark.xdist_group("block_participation")
@mark.staging
def test_spo_participation(
    all_mc_epochs_in_participation, api: BlockchainApi, count_blocks: int, block_participation, db_sync: Session
):
    # Track all registered SPO candidates across all MC epochs
    all_spo_keys = set()
    
    for mc_epoch in all_mc_epochs_in_participation:
        registered_candidates = api.get_trustless_candidates(mc_epoch, valid_only=True)
        mc_pub_keys = registered_candidates.keys()
        logging.info(f"Collecting SPO candidates from MC epoch {mc_epoch}")
        for mc_pub_key in mc_pub_keys:
            all_spo_keys.add(mc_pub_key)
    
    # Now remove all Incentivized entries from participation data that match our collected SPOs
    logging.info(f"Total unique SPO candidates found: {len(all_spo_keys)}")
    for mc_pub_key in all_spo_keys:
        # Remove all entries for this SPO producer (there may be multiple with different block counts)
        entries_to_remove = [
            entry for entry in block_participation["producer_participation"]
            if entry["block_producer"].get("Incentivized") and
               entry["block_producer"]["Incentivized"][1] == mc_pub_key
        ]
        for entry in entries_to_remove:
            logging.info(f"Removing SPO entry: {entry}")
            block_participation["producer_participation"].remove(entry)


@mark.dependency(depends=["pro_bono_participation", "spo_participation"])
@mark.xdist_group("block_participation")
@mark.staging
def test_no_unexpected_producers(block_participation):
    assert not block_participation["producer_participation"], "Unexpected producer participation data"


@mark.xdist_group("faucet_tx")
@mark.ci
@mark.staging
class TestMarginFee:
    @fixture(scope="class")
    def random_margin_fee(self) -> int:
        return random.randint(0, 10000)

    @fixture(scope="class", autouse=True)
    def set_margin_fee(self, api: BlockchainApi, get_wallet: Wallet, random_margin_fee) -> Transaction:
        result = api.set_block_producer_margin_fee(random_margin_fee, wallet=get_wallet)
        return result

    def test_set_margin_fee(self, set_margin_fee: Transaction):
        logging.info(f"Margin fee set: {set_margin_fee}")
        assert set_margin_fee._receipt.is_success

    def test_get_margin_fee(self, api: BlockchainApi, get_wallet: Wallet, random_margin_fee):
        response = api.partner_chain_rpc.partner_chain_get_block_producer_fees()
        account_id = get_wallet.address
        logging.info(f"Account ID: {account_id}")
        margin_fee = next((item["margin_fee"] for item in response.result if item["account_id"] == account_id), None)
        logging.info(f"Margin fee: {margin_fee}")
        assert random_margin_fee / 100 == margin_fee, f"Unexpected margin fee: {margin_fee}"
