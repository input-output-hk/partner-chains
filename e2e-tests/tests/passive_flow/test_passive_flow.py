import logging
from src.blockchain_api import BlockchainApi
from config.api_config import ApiConfig
from src.db.models import IncomingTx
from sqlalchemy.orm import Session
from sqlalchemy import Sequence
from pytest import mark
import pytest
from src.partner_chains_node.node import PartnerChainsNodeException


def get_burn_tx_from_rpc(api: BlockchainApi, tx_hash):
    txs_awaiting_mc_stability = api.get_incoming_txs()["awaitingMcStability"]
    return next((tx for tx in txs_awaiting_mc_stability if tx["txHash"] == tx_hash), None)


@mark.test_key('ETCM-7008')
@mark.passive_flow
def test_burn_was_successful(burn_tx: tuple[bool, IncomingTx]):
    """Test that it is possible to burn mainchain token

    * burn 1 mainchain token
    * check if result is successful
    """
    result, _ = burn_tx
    assert result


@mark.test_key('ETCM-7009')
@mark.passive_flow
def test_burn_amount_was_deducted_from_mc_addr(api: BlockchainApi, config: ApiConfig, burn_tx: tuple[bool, IncomingTx]):
    """Test that mainchain balance is updated after burn

    * burn 1 mainchain token
    * check that mainchain balance decreased by 1
    """
    _, tx = burn_tx
    actual_mc_balance = api.get_mc_balance(tx.mc_addr, config.nodes_config.token_policy_id)
    assert actual_mc_balance == tx.mc_balance - tx.amount


@mark.test_key('ETCM-7013')
@mark.passive_flow
def test_burn_tx_is_awaiting_mc_stability(
    api: BlockchainApi, burn_tx: tuple[bool, IncomingTx], config: ApiConfig, wait_until
):
    """Test that valid burn transaction appears in the sidechain_get_incoming_transactions() after burn

    * burn 1 mainchain token
    * wait till sidechain_get_incoming_transactions() contain burn transaction
    * check that response contains valid recipient, value and stableAtMainchainBlock
    """
    _, tx = burn_tx
    tx_from_rpc = wait_until(get_burn_tx_from_rpc, api, tx.tx_hash, timeout=config.timeouts.burn_tx_visible_in_pc_rpc)
    assert tx_from_rpc, "Burn tx not found in RPC"
    assert tx_from_rpc["recipient"] == tx.pc_addr
    assert tx_from_rpc["value"] == tx.amount
    assert (
        abs(tx_from_rpc["stableAtMainchainBlock"] - tx.stable_at_block) <= 1
    ), f"tx should be stable at block {tx.stable_at_block}, but rpc returned {tx_from_rpc['stableAtMainchainBlock']}"


@mark.test_key('ETCM-7014')
@mark.passive_flow
def test_burn_tx_was_received_on_pc_addr(
    incoming_txs_to_settle: Sequence[IncomingTx],
    api: BlockchainApi,
    config: ApiConfig,
    db: Session,
    pc_balance_since_last_settlement,
):
    """Test that sidechain balance increases after burning 1 token on the mainchain"""
    tokens_burnt = 0
    transaction: IncomingTx
    for transaction in incoming_txs_to_settle:
        logging.info(f"Settling balance of {transaction}")
        tokens_burnt += transaction.amount
    logging.info(f"Tokens burnt since last settlement {tokens_burnt}")

    if not pc_balance_since_last_settlement:
        pc_balance_since_last_settlement = incoming_txs_to_settle[0].pc_balance
    logging.info(f"PC balance since last settlement {pc_balance_since_last_settlement}")

    converted_tokens_burnt = tokens_burnt * 10**config.nodes_config.token_conversion_rate
    current_pc_balance = api.get_pc_balance(config.nodes_config.passive_transfer_account.partner_chain_address)

    assert current_pc_balance == pc_balance_since_last_settlement + converted_tokens_burnt

    for transaction in incoming_txs_to_settle:
        transaction.is_settled = True
        transaction.pc_balance_after_settlement = current_pc_balance
        db.commit()


##############################################################
#                      NEGATIVE TESTS                        #
##############################################################


@mark.test_key('ETCM-7010')
@mark.passive_flow
def test_burn_more_than_balance_should_fail(api: BlockchainApi, config: ApiConfig):
    """Test that trying to burn more than balance should fail

    * attempt burn more than mc balance
    * check that an exception is raised
    """
    mc_address = config.nodes_config.passive_transfer_account.mainchain_address
    amount = api.get_mc_balance(mc_address, config.nodes_config.token_policy_id) + 1
    burning_key = config.nodes_config.passive_transfer_account.mainchain_key
    pc_addr = config.nodes_config.negative_test_transfer_account.partner_chain_address

    with pytest.raises(PartnerChainsNodeException) as excinfo:
        api.burn_tokens(pc_addr, amount, burning_key)
    assert "BalanceInsufficientError" in excinfo.value.message


@mark.test_key('ETCM-7011')
@mark.passive_flow
def test_burn_negative_balance_should_fail(api: BlockchainApi, config: ApiConfig):
    """Test that trying to burn negative balance should fail

    * attempt to burn negative balance
    * check that an exception is raised
    """
    amount = -1
    burning_key = config.nodes_config.passive_transfer_account.mainchain_key
    pc_addr = config.nodes_config.negative_test_transfer_account.partner_chain_address

    with pytest.raises(PartnerChainsNodeException) as excinfo:
        api.burn_tokens(pc_addr, amount, burning_key)
    assert "ExUnitsEvaluationFailed" in excinfo.value.message


@mark.test_key('ETCM-7012')
@mark.passive_flow
def test_burn_with_invalid_key_should_fail(api: BlockchainApi, config: ApiConfig):
    """Test that trying to burn with invalid (malformed cbor) key should fail

    * attempt to burn with invalid skey
    * Expected error with message "Error while decoding key"
    """
    amount = 1
    burning_key = config.nodes_config.invalid_mc_skey.mainchain_key
    pc_addr = config.nodes_config.negative_test_transfer_account.partner_chain_address

    with pytest.raises(PartnerChainsNodeException) as excinfo:
        api.burn_tokens(pc_addr, amount, burning_key)
    assert "Error while decoding key" in excinfo.value.message


@mark.test_key('ETCM-7149')
@mark.passive_flow
def test_burn_with_receiver_address_less_than_32_bytes(api: BlockchainApi, config: ApiConfig):
    """Test that setting the receiver address to a value that is less than 32 bytes does not
    succeed and doesn't break the chain
    * attempt to burn with a pc_addr less than 32 bytes long
    * check that an exception is raised
    * check that rpc remains functional
    """
    burning_key = config.nodes_config.passive_transfer_account.mainchain_key
    pc_addr = config.nodes_config.negative_test_transfer_account.partner_chain_address

    amount = 12
    recipient_hex = api.address_to_hex(pc_addr)
    recipient_hex = recipient_hex[2:]

    api.burn_tokens_for_hex_address(recipient_hex, amount, burning_key)
    response = api.get_incoming_txs()

    assert response is not None, "Get incoming tx rpc endpoint does not return data"

    found = any(
        transaction
        for transaction in response['awaitingMcStability']
        if recipient_hex in transaction['recipient'] and transaction['value'] == amount
    )

    assert found, "No matching recipient and value found in 'awaitingMcStability'"


@mark.test_key('ETCM-7148')
@mark.passive_flow
def test_burn_with_receiver_address_more_than_32_bytes(api: BlockchainApi, config: ApiConfig):
    """Test that setting the receiver address to a value that is more than 32 bytes does not
    succeed and doesn't break the chain
    * attempt to burn with a pc_addr more than 32 bytes long
    * check that an exception is raised
    * check that rpc remains functional
    """
    burning_key = config.nodes_config.passive_transfer_account.mainchain_key
    pc_addr = config.nodes_config.negative_test_transfer_account.partner_chain_address

    amount = 21

    recipient_hex = api.address_to_hex(pc_addr)
    recipient_hex = recipient_hex + "11"

    api.burn_tokens_for_hex_address(recipient_hex, amount, burning_key)

    response = api.get_incoming_txs()

    assert response is not None, "Get incoming tx rpc endpoint does not return data"

    found = any(
        transaction
        for transaction in response['awaitingMcStability']
        if recipient_hex in transaction['recipient'] and transaction['value'] == amount
    )

    assert found, "No matching recipient and value found in 'awaitingMcStability'"
