import logging
from config.api_config import ApiConfig
from src.blockchain_api import BlockchainApi
from src.pc_epoch_calculator import PartnerChainEpochCalculator
from sqlalchemy.orm import Session
from sqlalchemy.sql import text

PARTICIPATION_DATA_SLOT_RANGE = 30


def test_delegator_rewards(
    api: BlockchainApi,
    config: ApiConfig,
    pc_epoch_calculator: PartnerChainEpochCalculator,
    current_mc_epoch: int,
    db_sync: Session,
):
    pool_hash_id = api.get_block_participation_data()
    up_to_slot = pool_hash_id["up_to_slot"]
    logging.info(f"up_to_slot: {up_to_slot}")
    first_epoch = (up_to_slot - PARTICIPATION_DATA_SLOT_RANGE) // config.nodes_config.slots_in_epoch
    last_epoch = up_to_slot // config.nodes_config.slots_in_epoch
    logging.info(f"first_epoch: {first_epoch}")
    logging.info(f"last_epoch: {last_epoch}")
    first_mc_epoch = pc_epoch_calculator.find_mc_epoch(first_epoch, current_mc_epoch)
    last_mc_epoch = pc_epoch_calculator.find_mc_epoch(last_epoch, current_mc_epoch)
    logging.info(f"first_mc_epoch: {first_mc_epoch}")
    logging.info(f"last_mc_epoch: {last_mc_epoch}")
    logging.info(f"current_mc_epoch: {current_mc_epoch}")

    for mc_epoch in range(first_mc_epoch, last_mc_epoch + 1):
        logging.info(f"Verifying main chain epoch: {mc_epoch}")
        spo_list = api.get_ariadne_parameters(mc_epoch)["candidateRegistrations"].keys()
        logging.info(f"Registered SPOs: {spo_list}")
        for spo in spo_list:
            logging.info(f"spo: {spo}")
            stake_pool_id = api.cardano_cli.get_stake_pool_id(cold_vkey=spo[2:], output_format="bech32")
            logging.info(f"stake_pool_id: {stake_pool_id}")
            query = text(
                "SELECT sa.view AS stake_address, encode(sa.hash_raw, 'hex') AS stake_hash, es.amount AS stake_amount "
                "FROM epoch_stake es "
                "JOIN stake_address sa ON es.addr_id = sa.id "
                f"WHERE es.pool_id = (SELECT id FROM pool_hash WHERE view = '{stake_pool_id}') "
                f"AND es.epoch_no = {mc_epoch} "
                "ORDER BY es.epoch_no DESC;"
            )
            result = db_sync.execute(query)
            for row in result:
                logging.info(f"DBSync result: {row}")
