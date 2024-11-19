from time import sleep
from pytest import mark
from src.blockchain_api import BlockchainApi, Wallet, Transaction
from config.api_config import ApiConfig
import logging as logger


@mark.CD
class TestSmoke:
    @mark.ariadne
    @mark.substrate
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

    @mark.ariadne
    @mark.substrate
    @mark.test_key('ETCM-6993')
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

    @mark.test_key('ETCM-6994')
    @mark.ariadne
    @mark.rpc
    def test_get_status(self, api: BlockchainApi):
        """Test partner_chain_getStatus() has same data as cardano-cli query tip

        * execute partner_chain_getStatus() API call
        * get mainchain epoch and slot from cardano-cli
        * check that mainchain slot from getStatus() is equal to slot from cardano-cli
        * check nextEpochTimestamp from getStatus()
        """
        expected_mc_epoch = api.get_mc_epoch()
        expected_mc_slot = api.get_mc_slot()

        partner_chain_status = api.get_status()
        assert partner_chain_status['mainchain']['epoch'] == expected_mc_epoch

        SLOT_DIFF_TOLERANCE = 100
        assert abs(partner_chain_status['mainchain']['slot'] - expected_mc_slot) <= SLOT_DIFF_TOLERANCE
        logger.info(
            f"Slot difference between getStatus() and cardano_cli tip is \
            {abs(partner_chain_status['mainchain']['slot'] - expected_mc_slot)}"
        )

        assert partner_chain_status['mainchain']['nextEpochTimestamp']
        assert partner_chain_status['sidechain']['nextEpochTimestamp']
        assert partner_chain_status['sidechain']['epoch']
        assert partner_chain_status['sidechain']['slot']

    @mark.ariadne
    @mark.rpc
    @mark.test_key('ETCM-7442')
    def test_get_params(self, api: BlockchainApi, config: ApiConfig):
        """Test partner_chain_getParams() returns proper values

        * execute partner_chain_getParams() API call
        * check that the return data is equal to the config values
        """
        params = api.get_params()
        assert params["genesis_utxo"] == config.genesis_utxo, "Genesis UTXO mismatch"
