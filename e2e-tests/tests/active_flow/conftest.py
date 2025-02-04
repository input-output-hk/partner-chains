from pytest import fixture, skip
from sqlalchemy import select, desc
from sqlalchemy.orm import Session
from src.db.models import OutgoingTx
from src.blockchain_api import BlockchainApi
from config.api_config import ApiConfig


@fixture(scope="session")
def mc_active_address(config: ApiConfig):
    return config.nodes_config.active_transfer_account.mainchain_address


@fixture
def unclaimed_outgoing_txs(api: BlockchainApi, db: Session, config: ApiConfig) -> OutgoingTx:
    current_pc_epoch = api.get_pc_epoch()

    query = (
        select(OutgoingTx)
        .where(OutgoingTx.is_claimed.isnot(True))
        .where(OutgoingTx.available_on_pc_epoch < current_pc_epoch)
        .where(OutgoingTx.tx_index_on_pc_epoch.isnot(None))
        .where(OutgoingTx.token_policy_id == config.nodes_config.token_policy_id)
    )
    txs_to_claim = db.scalars(query).all()

    if not txs_to_claim:
        skip("No unclaimed txs ready to be claimed")

    return txs_to_claim


@fixture
def latest_claimed_outgoing_tx(db: Session, config: ApiConfig) -> OutgoingTx:

    query = (
        select(OutgoingTx)
        .where(OutgoingTx.is_claimed)
        .where(OutgoingTx.is_received)
        .where(OutgoingTx.token_policy_id == config.nodes_config.token_policy_id)
        .order_by(desc(OutgoingTx.id))
        .limit(1)
    )
    tx_to_claim = db.scalars(query).one()

    if not tx_to_claim:
        skip("No claimed transaction in the database")

    return tx_to_claim


@fixture
def pending_outgoing_txs(db: Session, config: ApiConfig) -> OutgoingTx:
    query = (
        select(OutgoingTx)
        .where(OutgoingTx.is_claimed.isnot(True))
        .where(OutgoingTx.available_on_pc_epoch.isnot(None))
        .where(OutgoingTx.token_policy_id == config.nodes_config.token_policy_id)
    )
    txs_to_verify_as_pending = db.scalars(query).all()
    if not txs_to_verify_as_pending:
        skip("No pending txs available to retrieve index")

    return txs_to_verify_as_pending


@fixture
def unverified_outgoing_txs(db: Session, config: ApiConfig) -> OutgoingTx:
    query = (
        select(OutgoingTx)
        .where(OutgoingTx.is_claimed)
        .where(OutgoingTx.is_received == None)
        .where(OutgoingTx.token_policy_id == config.nodes_config.token_policy_id)
    )
    txs_to_balance = db.scalars(query).all()

    if not txs_to_balance:
        skip("No claimed txs need to be balanced")

    return txs_to_balance
