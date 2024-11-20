import logging
from src.blockchain_api import BlockchainApi, Transaction
from config.api_config import ApiConfig
from src.db.models import OutgoingTx
from sqlalchemy.orm import Session
from pytest import mark, raises
from src.pc_contracts_cli import PCContractsCliException


@mark.skip_blockchain("pc_evm", reason="not implemented yet")
@mark.test_key('ETCM-6996')
@mark.active_flow
def test_lock_transaction(api: BlockchainApi, config: ApiConfig, db: Session, secrets, mc_active_address):
    """Test that the user can lock tokens on a partner chain

    * create new transaction
    * lock transaction by calling lock() from ActiveFlow module
    * sign and submit transaction by calling extrinsic methods from substrate API
    """
    logging.info("Creating and submitting lock tx for active flow")
    # get spending wallet
    sender_wallet = get_wallet(api, secrets["wallets"]["active-flow"])
    # create transaction
    tx = Transaction()
    tx.sender = sender_wallet.address
    tx.recipient = mc_active_address
    tx.value = 1 * 10**config.nodes_config.token_conversion_rate

    db_tx = OutgoingTx()
    db_tx.pc_addr = tx.sender
    # To get balance we need the string format of the address
    db_tx.mc_addr = config.nodes_config.active_transfer_account.mainchain_address
    db_tx.token_policy_id = config.nodes_config.token_policy_id
    db_tx.amount = tx.value
    db_tx.pc_balance = api.get_pc_balance(tx.sender)
    db_tx.mc_balance = api.get_mc_balance(db_tx.mc_addr, config.nodes_config.token_policy_id)
    db.add(db_tx)
    db.commit()

    tx = api.lock_transaction(tx)

    # sign and submit transaction
    signed = api.sign_transaction(tx=tx, wallet=sender_wallet)
    api.submit_transaction(tx=signed, wait_for_finalization=True)
    current_pc_epoch = api.get_pc_epoch()
    db_tx.available_on_pc_epoch = current_pc_epoch if api.get_pc_epoch_phase() == "regular" else current_pc_epoch + 1
    db_tx.lock_tx_hash = tx.hash
    db_tx.pc_balance_after_lock = api.get_pc_balance(tx.sender)
    db_tx.fees_spent = db_tx.pc_balance - db_tx.pc_balance_after_lock - tx.value
    db.commit()
    expected_fee = api.get_expected_tx_fees(sender_wallet.crypto_type, 'lock')
    assert (
        abs(db_tx.fees_spent - expected_fee) <= 1
    ), f"Actual fees do not match expected value from config: {db_tx.fees_spent} != {expected_fee}"
    assert (
        db_tx.pc_balance - tx.value - db_tx.fees_spent == db_tx.pc_balance_after_lock
    ), f"Balance mismatch: {db_tx.pc_balance_after_lock} != {db_tx.pc_balance} - {tx.value} - {db_tx.fees_spent}"


@mark.skip_blockchain("pc_evm", reason="not implemented yet")
@mark.test_key('ETCM-7002')
@mark.active_flow
def test_outgoing_transaction_is_pending(
    api: BlockchainApi,
    config: ApiConfig,
    db: Session,
    pending_outgoing_txs: OutgoingTx,
):
    """Test that locked transactions is marked as outgoing (pending)

    * get outgoing transactions by calling OutgoingTransactions method from ActiveFlow module
    * verify that test db contains one of the outgoing transactions
    """
    # First pass to identify already used tx indices in db
    used_tx_index = {}
    for db_tx in pending_outgoing_txs:
        recipient_bech32 = api.cardano_address_to_bech32(db_tx.mc_addr)
        if db_tx.available_on_pc_epoch not in used_tx_index:
            used_tx_index[db_tx.available_on_pc_epoch] = {}
        if db_tx.tx_index_on_pc_epoch is not None:
            if (db_tx.amount, recipient_bech32) not in used_tx_index[db_tx.available_on_pc_epoch]:
                used_tx_index[db_tx.available_on_pc_epoch][(db_tx.amount, recipient_bech32)] = []
            used_tx_index[db_tx.available_on_pc_epoch][(db_tx.amount, recipient_bech32)].append(
                db_tx.tx_index_on_pc_epoch
            )

    # Second pass to assign the tx indices
    for db_tx in pending_outgoing_txs:
        if db_tx.tx_index_on_pc_epoch is not None:
            continue
        outgoing_txs = api.get_outgoing_transactions(db_tx.available_on_pc_epoch)
        for outgoing_tx in outgoing_txs:
            found_tx = False
            # Check if this index has already been assigned
            # Covers the case that amount and recipient are the same
            locked_amount = outgoing_tx['value'] * 10**config.nodes_config.token_conversion_rate
            if (
                locked_amount,
                outgoing_tx['recipient'],
            ) not in used_tx_index[db_tx.available_on_pc_epoch]:
                used_tx_index[db_tx.available_on_pc_epoch][(locked_amount, outgoing_tx['recipient'])] = []
            if locked_amount == db_tx.amount and outgoing_tx['recipient'] == recipient_bech32:
                if (
                    outgoing_tx['txIndex']
                    in used_tx_index[db_tx.available_on_pc_epoch][(locked_amount, outgoing_tx['recipient'])]
                ):
                    continue
                else:
                    used_tx_index[db_tx.available_on_pc_epoch][(locked_amount, outgoing_tx['recipient'])].append(
                        outgoing_tx['txIndex']
                    )
                db_tx.tx_index_on_pc_epoch = outgoing_tx['txIndex']
                db.commit()
                found_tx = True
                break
        assert (
            found_tx
        ), f"Tx {db_tx.lock_tx_hash} not found pending as expected on pc epoch {db_tx.available_on_pc_epoch}"


@mark.skip_blockchain("pc_evm", reason="not implemented yet")
@mark.test_key('ETCM-7005')
@mark.active_flow
def test_claim_transactions(
    api: BlockchainApi, config: ApiConfig, db: Session, unclaimed_outgoing_txs: OutgoingTx, wait_until
):
    """Test that the user can claim tokens after committee handover

    * wait until the next committee handover for a partner chain epoch
    * claim tokens by executing claimTokens() from pc-contracts-cli
    """
    for db_tx in unclaimed_outgoing_txs:
        logging.info(f"Checking if committee handover happened for epoch {db_tx.available_on_pc_epoch}")
        wait_until(api.check_epoch_signatures_uploaded, db_tx.available_on_pc_epoch, timeout=config.timeouts.claim_cmd)
        logging.info(f"Handover for pc epoch {db_tx.available_on_pc_epoch} has happened")
        logging.info(f"Claiming tx {db_tx}")
        db_tx.mc_balance_before_claim = api.get_mc_balance(db_tx.mc_addr, config.nodes_config.token_policy_id)
        outgoing_tx = api.get_outgoing_tx_merkle_proof(db_tx.available_on_pc_epoch, db_tx.tx_index_on_pc_epoch)
        db_tx.combined_proof = outgoing_tx['proof']['bytes'][2:]
        if "currentDistributedSetUtxo" in outgoing_tx:
            distributed_set_utxo = outgoing_tx['currentDistributedSetUtxo']
        else:
            distributed_set_utxo = None
        assert api.claim_tokens(
            config.nodes_config.active_transfer_account.mainchain_key,
            db_tx.combined_proof,
            distributed_set_utxo,
        ), f"Could not claim tx {db_tx.lock_tx_hash}"
        db_tx.is_claimed = True
        db_tx.mc_balance_after_claim = api.get_mc_balance(db_tx.mc_addr, config.nodes_config.token_policy_id)
        db.commit()


@mark.test_key('ETCM-7007')
@mark.active_flow
def test_verify_outgoing_balances(config: ApiConfig, db: Session, unverified_outgoing_txs: OutgoingTx):
    """Test that the user balances on mainchain and partner chain has been changed after claim"""
    for db_tx in unverified_outgoing_txs:
        db_tx.is_received = False
        db.commit()
        assert (
            db_tx.mc_balance_after_claim
            == db_tx.mc_balance_before_claim + db_tx.amount / 10**config.nodes_config.token_conversion_rate
        ), "MC balance mismatch"
        assert db_tx.pc_balance_after_lock == db_tx.pc_balance - db_tx.amount - db_tx.fees_spent, "PC balance mismatch"
        db_tx.is_received = True
        db.commit()


##############################################################
#                      NEGATIVE TESTS                        #
##############################################################


@mark.xfail(run=False, reason="[ETCM-6842] Locking less than the conversion ratio in tokens does not raise exception")
@mark.skip_blockchain("pc_evm", reason="not implemented yet")
@mark.test_key('ETCM-6997')
@mark.active_flow
def test_lock_less_than_conversion_ratio_tokens_should_fail(
    api: BlockchainApi, config: ApiConfig, secrets, mc_active_address
):
    """Test that the user can not lock less than conversion ratio in tokens

    * create new transaction with a value less than 1 but greater than 0
    * sign the transaction
    * submitting the transaction should raise an error
    """
    # get spending wallet
    sender_wallet = get_wallet(api, secrets["wallets"]["negative-test"])

    # create transaction
    tx = Transaction()
    tx.sender = sender_wallet.address
    tx.recipient = mc_active_address
    tx.value = 0.5 * 10**config.nodes_config.token_conversion_rate

    tx = api.lock_transaction(tx)

    signed = api.sign_transaction(tx=tx, wallet=sender_wallet)

    with raises(Exception) as excinfo:
        api.submit_transaction(tx=signed, wait_for_finalization=True)
    assert (
        str(excinfo.value)
        == "{'code': 1010, 'message': 'Invalid Transaction', 'data': 'Inability to pay some fees (e.g. account balance too low)'}"
    )


@mark.skip_blockchain("pc_evm", reason="not implemented yet")
@mark.test_key('ETCM-6998')
@mark.active_flow
def test_lock_with_empty_wallet_should_fail(api: BlockchainApi, config: ApiConfig, mc_active_address):
    """Test that the user can not lock tokens with the an empty wallet

    * create new transaction
    * sign the transaction with an empty wallet
    * submitting the transaction should raise an error
    """

    empty_wallet = api.new_wallet()

    # create transaction
    tx = Transaction()
    tx.sender = empty_wallet.address
    tx.recipient = mc_active_address
    tx.value = 1 * 10**config.nodes_config.token_conversion_rate

    tx = api.lock_transaction(tx)

    signed = api.sign_transaction(tx=tx, wallet=empty_wallet)

    with raises(Exception) as excinfo:
        api.submit_transaction(tx=signed, wait_for_finalization=True)
    assert (
        str(excinfo.value)
        == "{'code': 1010, 'message': 'Invalid Transaction', 'data': 'Inability to pay some fees (e.g. account balance too low)'}"
    )


@mark.skip_blockchain("pc_evm", reason="not implemented yet")
@mark.test_key('ETCM-6999')
@mark.active_flow
def test_lock_with_invalid_key_should_fail(api: BlockchainApi, config: ApiConfig, secrets, mc_active_address):
    """Test that the user can not lock tokens with the an invalid signing key

    * create new transaction
    * sign the transaction with an invalid key
    * submitting the transaction should raise an error
    """
    # get spending wallet
    sender_wallet = get_wallet(api, secrets["wallets"]["negative-test"])

    sender_wallet.private_key = sender_wallet.private_key + b"a"

    # create transaction
    tx = Transaction()
    tx.sender = sender_wallet.address
    tx.recipient = mc_active_address
    tx.value = 1 * 10**config.nodes_config.token_conversion_rate

    tx = api.lock_transaction(tx)

    signed = api.sign_transaction(tx=tx, wallet=sender_wallet)
    try:
        api.submit_transaction(tx=signed, wait_for_finalization=True)
        assert False, "Transaction was locked with an invalid signing key: This test should have failed"
    except Exception as e:
        assert True, e


@mark.xfail(run=False, reason="[ETCM-6843] Locking more than mc balance does not raise an exception")
@mark.skip_blockchain("pc_evm", reason="not implemented yet")
@mark.test_key('ETCM-7000')
@mark.active_flow
def test_lock_more_than_balance_should_fail(api: BlockchainApi, config: ApiConfig, secrets, mc_active_address):
    """Test that the user can not lock more tokens than mc balance on a partner chain

    * create new transaction
    * get mainchain balance + 1 as the transaction value
    * lock transaction by calling lock() from ActiveFlow module
    * submitting transaction should fail
    """
    # get spending wallet
    sender_wallet = get_wallet(api, secrets["wallets"]["negative-test"])

    # create transaction
    tx = Transaction()
    tx.sender = sender_wallet.address
    tx.recipient = mc_active_address
    tx.value = 1 * 10**config.nodes_config.token_conversion_rate

    tx = api.lock_transaction(tx)

    signed = api.sign_transaction(tx=tx, wallet=sender_wallet)
    with raises(AssertionError) as excinfo:
        api.submit_transaction(tx=signed, wait_for_finalization=True)
    assert (
        str(excinfo.value)
        == "{'code': 1010, 'message': 'Invalid Transaction', 'data': 'Inability to pay some fees (e.g. account balance too low)'}"
    )


@mark.skip_blockchain("pc_evm", reason="not implemented yet")
@mark.test_key('ETCM-7001')
@mark.active_flow
def test_lock_negative_balance_should_fail(api: BlockchainApi, config: ApiConfig, secrets, mc_active_address):
    """Test that the user can not lock a negative amount of mainchain tokens

    * create new transaction with a negative balance
    * locking transaction should fail
    """
    sender_wallet = get_wallet(api, secrets["wallets"]["negative-test"])

    tx = Transaction()
    tx.sender = sender_wallet.address
    tx.recipient = mc_active_address
    tx.value = (-1) * 10**config.nodes_config.token_conversion_rate

    with raises(Exception) as excinfo:
        api.lock_transaction(tx)
    assert str(excinfo.value) == "can't convert negative int to unsigned"


@mark.skip_blockchain("pc_evm", reason="not implemented yet")
@mark.test_key('ETCM-7003')
@mark.active_flow
def test_claim_transaction_signed_by_another_recipient_should_fail(
    api: BlockchainApi, config: ApiConfig, unclaimed_outgoing_txs: OutgoingTx, wait_until
):
    """Test that the user can not claim tokens signed by another recipient

    * get first unclaimed outgoing transaction
    * get it's merkle proof
    * wait until tokens are claimable
    * claim tokens using another recipient's key
    * Expected err msg: ERROR-FUEL-MINTING-POLICY-04: tx not signed by recipient
    """
    db_tx = unclaimed_outgoing_txs[0]
    logging.info(f"Claiming tx {db_tx}")
    outgoing_tx = api.get_outgoing_tx_merkle_proof(db_tx.available_on_pc_epoch, db_tx.tx_index_on_pc_epoch)
    db_tx.combined_proof = outgoing_tx['proof']['bytes'][2:]
    logging.info(f"Checking if committee handover happened for epoch {db_tx.available_on_pc_epoch}")
    wait_until(api.check_epoch_signatures_uploaded, db_tx.available_on_pc_epoch, timeout=config.timeouts.claim_cmd)
    logging.info(f"Handover for pc epoch {db_tx.available_on_pc_epoch} has happened")
    with raises(PCContractsCliException) as excinfo:
        api.claim_tokens(config.nodes_config.random_mc_account.mainchain_key, db_tx.combined_proof)
    assert "ERROR-FUEL-MINTING-POLICY-04" in excinfo.value.message


@mark.skip_blockchain("pc_evm", reason="not implemented yet")
@mark.test_key('ETCM-7004')
@mark.active_flow
def test_claim_transaction_with_invalid_key_should_fail(
    api: BlockchainApi, config: ApiConfig, unclaimed_outgoing_txs: OutgoingTx, wait_until
):
    """Test that the user can not claim tokens with invalid (malformed) signing key

    * get first unclaimed outgoing transaction
    * get it's merkle proof
    * wait until tokens are claimable
    * claim tokens using another recipient's key
    * Expected err msg: Error while decoding key
    """
    db_tx = unclaimed_outgoing_txs[0]
    logging.info(f"Claiming tx {db_tx}")
    outgoing_tx = api.get_outgoing_tx_merkle_proof(db_tx.available_on_pc_epoch, db_tx.tx_index_on_pc_epoch)
    db_tx.combined_proof = outgoing_tx['proof']['bytes'][2:]
    logging.info(f"Checking if committee handover happened for epoch {db_tx.available_on_pc_epoch}")
    wait_until(api.check_epoch_signatures_uploaded, db_tx.available_on_pc_epoch, timeout=config.timeouts.claim_cmd)
    logging.info(f"Handover for pc epoch {db_tx.available_on_pc_epoch} has happened")
    with raises(PCContractsCliException) as excinfo:
        api.claim_tokens(config.nodes_config.invalid_mc_skey.mainchain_key, db_tx.combined_proof)
    assert "Error while decoding key" in excinfo.value.message


@mark.test_key('ETCM-7006')
@mark.active_flow
def test_claim_on_already_claimed_transaction_should_fail(
    api: BlockchainApi, config: ApiConfig, latest_claimed_outgoing_tx: OutgoingTx
):
    """Test that the user can not claim the same tokens

    * Find the latest claimed outgoing tx in test database
    * claiming tokens for that transaction again should fail
    """
    logging.info(f"Claiming tx that should fail: {latest_claimed_outgoing_tx}")
    with raises(PCContractsCliException) as excinfo:
        api.claim_tokens(
            config.nodes_config.active_transfer_account.mainchain_key,
            latest_claimed_outgoing_tx.combined_proof,
        )
    assert "NotFoundUtxo" in excinfo.value.message


def get_wallet(api, config):
    return api.get_wallet(
        config["address"],
        config["public_key"],
        config["private_key"],
        config["scheme"],
    )
