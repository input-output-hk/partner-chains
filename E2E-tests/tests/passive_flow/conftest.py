from config.api_config import ApiConfig
from pytest import fixture, skip
from sqlalchemy import desc, select, Sequence
from sqlalchemy.orm import Session
from src.db.models import IncomingTx
from src.blockchain_api import BlockchainApi
from filelock import FileLock


def create_burn_tx(api, db, config):
    tx = IncomingTx()
    tx.addr = config.nodes_config.passive_transfer_account.partner_chain_address
    tx.mc_addr = config.nodes_config.passive_transfer_account.mainchain_address
    tx.pc_balance = api.get_pc_balance(tx.pc_addr)
    tx.mc_balance = api.get_mc_balance(tx.mc_addr, config.nodes_config.token_policy_id)
    tx.token_policy_id = config.nodes_config.token_policy_id
    tx.amount = 1
    db.add(tx)
    db.commit()
    result, tx_hash, mc_stable_block = api.burn_tokens(
        tx.pc_addr, tx.amount, config.nodes_config.passive_transfer_account.mainchain_key
    )
    if result:
        tx.tx_hash = tx_hash
        tx.stable_at_block = mc_stable_block
        db.commit()
    return result, tx


@fixture(scope="module")
def burn_tx(tmp_path_factory, worker_id, api: BlockchainApi, config: ApiConfig, db: Session) -> tuple[bool, IncomingTx]:
    if worker_id == "master":
        return create_burn_tx(api=api, db=db, config=config)

    root_tmp_dir = tmp_path_factory.getbasetemp().parent
    fn = root_tmp_dir / "burn.tx"
    with FileLock(str(fn) + ".lock"):
        if fn.is_file():
            data = fn.read_text().split(",")
            result = bool(data[0])
            tx_id = int(data[1])
            query = select(IncomingTx).where(IncomingTx.id == tx_id)
            tx = db.scalar(query)
        else:
            result, tx = create_burn_tx(api=api, db=db, config=config)
            fn.write_text(f"{result},{tx.id}")
    return result, tx


@fixture
def pc_balance_since_last_settlement(config: ApiConfig, db: Session) -> int | None:
    """Returns partner chain balance after last settlement."""
    query = (
        select(IncomingTx)
        .where(IncomingTx.is_settled)
        .where(IncomingTx.pc_addr == config.nodes_config.passive_transfer_account.partner_chain_address)
        .where(IncomingTx.token_policy_id == config.nodes_config.token_policy_id)
        .order_by(desc(IncomingTx.id))
        .limit(1)
    )
    last_settled_tx = db.scalar(query)

    if last_settled_tx:
        balance = last_settled_tx.pc_balance_after_settlement
    else:
        balance = None
    return balance


@fixture
def incoming_txs_to_settle(api: BlockchainApi, config: ApiConfig, db: Session) -> Sequence[IncomingTx]:
    """Returns list of incoming transactions that are stable and ready to settle. Skip the test if none were found."""
    current_mc_block = api.get_latest_mc_block_number()
    query = (
        select(IncomingTx)
        .where(IncomingTx.is_settled.is_not(True))
        .where(IncomingTx.stable_at_block <= current_mc_block)
        .where(IncomingTx.pc_addr == config.nodes_config.passive_transfer_account.partner_chain_address)
        .where(IncomingTx.token_policy_id == config.nodes_config.token_policy_id)
        .order_by(IncomingTx.id)
    )
    incoming_txs_to_settle = db.scalars(query).all()

    if not incoming_txs_to_settle:
        skip("No incoming transactions to settle")

    return incoming_txs_to_settle
