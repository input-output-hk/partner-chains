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
def block_to_query_storage(static_api: BlockchainApi):
    block = static_api.get_block()
    block_no = block["header"]["number"]
    if block_no <= PARTICIPATION_DATA_SLOT_RANGE:
        skip(f"Participation data is released after {PARTICIPATION_DATA_SLOT_RANGE} slots, current block {block_no}.")
    logging.info(f"Block to query storage: {block_no}")
    return block


@fixture(scope="module")
def block_participation(block_to_query_storage, static_api: BlockchainApi):
    block_participation = static_api.get_block_participation_data(block_hash=block_to_query_storage["header"]["hash"])
    return block_participation


@fixture(scope="module")
def block_production_log(block_to_query_storage, static_api: BlockchainApi):
    block_no = block_to_query_storage["header"]["number"]
    block_no_matching_participation_data = block_no - len(
        static_api.get_block_production_log(block_hash=block_to_query_storage["header"]["hash"])
    )
    logging.info(f"Block number matching participation data slots range: {block_no_matching_participation_data}")
    block_matching_participation_data = static_api.get_block(block_no_matching_participation_data)
    block_production_log = static_api.get_block_production_log(block_hash=block_matching_participation_data["header"]["hash"])
    return block_production_log


@fixture(scope="module")
def pc_epochs(block_participation, config: ApiConfig, initial_pc_epoch):
    up_to_slot = block_participation["up_to_slot"]
    logging.info(f"Participation data up to slot: {up_to_slot}")
    start_pc_epoch = (up_to_slot - PARTICIPATION_DATA_SLOT_RANGE) // config.nodes_config.slots_in_epoch
    stop_pc_epoch = up_to_slot // config.nodes_config.slots_in_epoch
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


@mark.dependency(name="participation_data")
@mark.xdist_group("block_participation")
def test_block_participation_data_is_not_empty(block_participation):
    assert block_participation
    assert block_participation["up_to_slot"]
    assert block_participation["producer_participation"]


@mark.dependency(name="pro_bono_participation")
@mark.xdist_group("block_participation")
def test_pro_bono_participation(
    mc_epochs: range, api: BlockchainApi, initial_pc_epoch_included, count_blocks: int, block_participation
):
    for mc_epoch in mc_epochs:
        logging.info(f"Verifying ProBono participation in MC epoch {mc_epoch}")
        permissioned_candidates = api.get_permissioned_candidates(mc_epoch, valid_only=True)

        initial_pc_epoch = initial_pc_epoch_included(mc_epoch)
        if initial_pc_epoch:
            logging.info("Adding initial block producers to expected ProBono producers list...")
            initial_block_producers = api.get_epoch_committee(initial_pc_epoch).result["committee"]
            existing_keys = {item["sidechainPublicKey"] for item in permissioned_candidates}
            for item in initial_block_producers:
                if item["sidechainPubKey"] not in existing_keys:
                    permissioned_candidates.append({"sidechainPublicKey": item["sidechainPubKey"]})

        for permissioned_candidate in permissioned_candidates:
            expected_producer = {}
            expected_producer["block_producer"] = {"ProBono": permissioned_candidate["sidechainPublicKey"]}
            expected_producer["block_count"] = count_blocks(mc_epoch, expected_producer["block_producer"])
            if expected_producer["block_count"] == 0:
                logging.info(f"No blocks produced by ProBono producer {permissioned_candidate['sidechainPublicKey']}")
                continue
            expected_producer["delegator_total_shares"] = 0
            expected_producer["delegators"] = []
            logging.info(f"Expected ProBono Producer: {expected_producer}")

            assert expected_producer in block_participation["producer_participation"]
            block_participation["producer_participation"].remove(expected_producer)


@mark.dependency(name="spo_participation")
@mark.xdist_group("block_participation")
def test_spo_participation(
    mc_epochs: range, api: BlockchainApi, count_blocks: int, block_participation, db_sync: Session
):
    for mc_epoch in mc_epochs:
        registered_candidates = api.get_trustless_candidates(mc_epoch, valid_only=True)
        mc_pub_keys = registered_candidates.keys()
        logging.info(f"Verifying SPO participation in MC epoch {mc_epoch}")
        for mc_pub_key in mc_pub_keys:
            expected_spo = {}
            assert len(registered_candidates[mc_pub_key]) == 1, "Multiple registrations with the same MC public key"

            pc_pub_key = registered_candidates[mc_pub_key][0]["sidechainPubKey"]
            expected_spo["block_producer"] = {"Incentivized": (pc_pub_key, mc_pub_key)}
            expected_spo["block_count"] = count_blocks(mc_epoch, expected_spo["block_producer"])
            if expected_spo["block_count"] == 0:
                logging.info(f"No blocks produced by SPO producer {mc_pub_key}")
                continue

            mc_epoch_for_stake = mc_epoch - 2
            stake_pool_id = api.cardano_cli.get_stake_pool_id(cold_vkey=mc_pub_key[2:], output_format="bech32")
            query = text(
                "SELECT sa.view AS stake_address, encode(sa.hash_raw, 'hex') AS stake_hash, es.amount AS stake_amount "
                "FROM epoch_stake es "
                "JOIN stake_address sa ON es.addr_id = sa.id "
                f"WHERE es.pool_id = (SELECT id FROM pool_hash WHERE view = '{stake_pool_id}') "
                f"AND es.epoch_no = {mc_epoch_for_stake} "
                "AND es.amount > 0;"
            )
            spdd = db_sync.execute(query)
            expected_spo["delegators"] = []
            expected_spo["delegator_total_shares"] = 0
            for delegator in spdd:
                logging.info(f"SPO: {mc_pub_key}, Delegator: {delegator}")
                expected_delegator = {}
                stake_key_hash = delegator._mapping["stake_hash"][2:]
                expected_delegator["id"] = {"StakeKeyHash": f"0x{stake_key_hash}"}
                expected_delegator["share"] = int(delegator._mapping["stake_amount"])
                expected_spo["delegators"].append(expected_delegator)
                expected_spo["delegator_total_shares"] += int(delegator._mapping["stake_amount"])

            logging.info(f"Expected SPO: {expected_spo}")

            assert expected_spo in block_participation["producer_participation"]
            block_participation["producer_participation"].remove(expected_spo)


@mark.dependency(depends=["pro_bono_participation", "spo_participation"])
@mark.xdist_group("block_participation")
def test_no_unexpected_producers(block_participation):
    assert not block_participation["producer_participation"], "Unexpected producer participation data"


@mark.xdist_group("faucet_tx")
class TestMarginFee:
    @fixture(scope="class")
    def random_margin_fee(self) -> int:
        return random.randint(0, 10000)

    @fixture(scope="class", autouse=True)
    def set_margin_fee(self, static_api: BlockchainApi, get_wallet: Wallet, random_margin_fee) -> Transaction:
        result = static_api.set_block_producer_margin_fee(random_margin_fee, wallet=get_wallet)
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
