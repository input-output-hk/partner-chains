from time import sleep
from pytest import mark
from src.blockchain_api import BlockchainApi, Wallet, Transaction
from config.api_config import ApiConfig
import logging as logger


@mark.smoke
class TestSmoke:
    @mark.test_key('ETCM-6992')
    def test_block_producing(self, api: BlockchainApi, config: ApiConfig):
        """Test node producing a block

        * get latest partner chain block
        * wait for a predefined time
        * get latest partner chain block one more time
        * verify that block numbers increased
        """
        block_number = api.get_latest_pc_block_number()
        sleep_time = 1.5 * config.nodes_config.block_duration
        logger.info(f"Waiting for new block {sleep_time} seconds...")
        sleep(sleep_time)
        assert api.get_latest_pc_block_number() > block_number

    @mark.test_key('ETCM-6993')
    @mark.xdist_group("faucet_tx")
    def test_transaction(self, api: BlockchainApi, new_wallet: Wallet, get_wallet: Wallet, config: ApiConfig):
        """Test node making a transaction

        * create a transaction
        * sign transaction
        * submit transaction
        * check a balance of receiver was updated
        """
        # create transaction
        value = 1 * 10**config.nodes_config.token_conversion_rate
        tx = Transaction()
        sender_wallet = get_wallet
        sender_balance_before = api.get_pc_balance(sender_wallet.address)

        tx.sender = sender_wallet.address
        tx.recipient = new_wallet.address
        tx.value = value
        tx = api.build_transaction(tx)

        # sign and submit transaction
        signed = api.sign_transaction(tx=tx, wallet=sender_wallet)
        api.submit_transaction(tx=signed, wait_for_finalization=True)

        # check new address' balance
        receiver_balance = api.get_pc_balance(new_wallet.address)

        sender_balance_after = api.get_pc_balance(sender_wallet.address)
        assert value == receiver_balance, "Receiver's balance mismatch"
        assert sender_balance_after == sender_balance_before - value - tx.total_fee_amount, "Sender's balance mismatch"
