import secrets
from web3 import Web3, Account
from .blockchain_api import BlockchainApi, Transaction, Wallet
from config.api_config import ApiConfig
from .decorators import long_running_function
import logging as logger


class PartnerChainEvmApi(BlockchainApi):
    def __init__(self, config: ApiConfig, secrets):
        self.config = config
        self.secrets = secrets
        self.url = config.nodes_config.node_url
        self._w3 = None

    @property
    def w3(self):
        if self._w3 is None:
            self._w3 = Web3(Web3.HTTPProvider(self.url))
        return self._w3

    def close(self):
        if self._w3:
            self._w3 = None

    def get_latest_pc_block_number(self):
        block = self.w3.eth.get_block("latest")
        logger.debug(f"Current block: {block}")
        return block["number"]

    def get_pc_balance(self, address):
        balance = self.w3.eth.get_balance(address)
        logger.debug(f"Address {address} balance: {balance}")
        return balance

    def build_transaction(self, tx: Transaction):
        tx._unsigned = {
            "nonce": self.w3.eth.get_transaction_count(tx.sender),
            "gasPrice": self.w3.eth.gas_price,
            "gas": 100000,
            "to": tx.recipient,
            "value": tx.value,
        }
        logger.debug(f"Transaction built {tx._unsigned}")
        return tx

    def sign_transaction(self, tx: Transaction, wallet: Wallet):
        tx._signed = self.w3.eth.account.sign_transaction(tx._unsigned, wallet.private_key)
        logger.debug(f"Transaction signed {tx._signed}")
        return tx

    @long_running_function
    def submit_transaction(self, tx: Transaction, wait_for_finalization):
        tx._receipt = self.w3.eth.send_raw_transaction(tx._signed.rawTransaction)
        tx.hash = tx._receipt.hex()
        logger.debug(f"Transaction sent {tx.hash}")
        if wait_for_finalization:
            self.w3.eth.wait_for_transaction_receipt(
                tx.hash,
                poll_latency=self.config.poll_intervals.transaction_finalization,
            )
        return tx

    def new_wallet(self):
        private_key = "0x" + secrets.token_hex(32)
        account = Account.from_key(private_key)
        wallet = Wallet()
        wallet.raw = account
        wallet.address = account.address
        wallet.private_key = private_key
        logger.debug(f"New wallet created {wallet.address}")
        return wallet

    def get_wallet(self, address=None, public_key=None, secret=None):
        if not address:
            address = self.secrets["wallets"]["faucet-0"]["address"]
        if not secret:
            secret = self.secrets["wallets"]["faucet-0"]["secret_seed"]
        wallet = Wallet()
        wallet.address = address
        wallet.private_key = secret
        return wallet

    def get_authorities(self):
        raise NotImplementedError
