from abc import ABC, abstractmethod
from src.partner_chain_rpc import PartnerChainRpcResponse, DParam
from src.partner_chains_node import AddressAssociationSignature
from config.api_config import Node


class Transaction:
    _unsigned = None  # raw tx object to be signed
    _signed = None  # raw tx_sign object to be submitted
    _receipt = None  # raw tx_receipt object after submitting
    sender: str
    recipient: str
    value: str
    hash: str
    total_fee_amount: str


class Wallet:
    raw = None
    address: str
    private_key: str
    mnemonic: str
    crypto_type: int
    seed: str
    public_key: str


class BlockchainApi(ABC):
    @abstractmethod
    def close(self):
        pass

    @abstractmethod
    def get_latest_pc_block_number(self) -> int:
        pass

    @abstractmethod
    def get_latest_mc_block_number(self) -> int:
        pass

    @abstractmethod
    def get_pc_epoch(self) -> int:
        pass

    @abstractmethod
    def get_pc_epoch_phase(self, slot_number=None) -> str:
        pass

    @abstractmethod
    def get_params(self) -> dict:
        pass

    @abstractmethod
    def get_mc_epoch(self) -> int:
        pass

    @abstractmethod
    def get_mc_slot(self) -> int:
        pass

    @abstractmethod
    def get_mc_block(self) -> int:
        pass

    @abstractmethod
    def get_mc_sync_progress(self) -> float:
        pass

    @abstractmethod
    def get_pc_balance(self, address) -> int:
        pass

    @abstractmethod
    def get_mc_balance(self, address, policy_id=None) -> int:
        pass

    @abstractmethod
    def wait_for_next_pc_block(self) -> bool:
        pass

    @abstractmethod
    def wait_for_next_mc_block(self) -> bool:
        pass

    @abstractmethod
    def get_outgoing_transactions(self, epoch) -> list:
        pass

    @abstractmethod
    def build_transaction(self, tx: Transaction) -> Transaction:
        pass

    @abstractmethod
    def sign_transaction(self, tx: Transaction, wallet: Wallet) -> Transaction:
        pass

    @abstractmethod
    def submit_transaction(self, tx: Transaction, wait_for_finalization=False) -> Transaction:
        pass

    @abstractmethod
    def lock_transaction(self, tx: Transaction) -> Transaction:
        pass

    @abstractmethod
    def new_wallet(self) -> Wallet:
        pass

    @abstractmethod
    def get_wallet(self, address, public_key, secret, scheme) -> Wallet:
        pass

    @abstractmethod
    def address_to_hex(self, address) -> str:
        pass

    @abstractmethod
    def get_authorities(self) -> list:
        pass

    @abstractmethod
    def get_status(self):
        pass

    @abstractmethod
    def update_d_param(self, permissioned_candidates_count: int, trustless_candidates_count: int) -> (bool, int):
        """
        Update D parameter configuration for the sidechain
        Arguments:
            permissioned_candidates_count {int} -- Number of permissioned candidates
            trustless_candidates_count {int} -- Number of trustless candidates
        Returns:
            (bool, int) - True/False, and a main chain epoch that it will take effect
        """
        pass

    @abstractmethod
    def register_candidate(self, candidate_name: str) -> (bool, int):
        """
        Registers candidate to participate in a partner chain consensus protocol

        Arguments:
            candidate_name {str} -- Candidate name. Has to be the same in config and db

        Returns:
            (bool, int) - True/False, and main chain epoch that it will take effect
        """
        pass

    @abstractmethod
    def deregister_candidate(self, candidate: str) -> (bool, int):
        """
        Deregisters candidate from participation in a partner chain consensus protocol

        Arguments:
            candidate_name {str} -- Candidate name. Has to be the same in config and db

        Returns:
            (bool, int) - True/False, and main chain epoch that it will take effect
        """
        pass

    @abstractmethod
    def upsert_permissioned_candidates(self, new_candidates_list: dict[str, Node]) -> (bool, int):
        pass

    @abstractmethod
    def burn_tokens(self, recipient: str, amount: int, payment_key: str) -> (bool, str, int):
        """Initiate transaction from main chain to partner chain, aka passive flow.

        Arguments:
            recipient {str} -- recipient's address
            amount {int} -- amount of tokens to burn on the main chain
            payment_key {str} -- main chain address' signing key

        Returns:
            (bool, string, int) - True/False, transaction hash, and main chain block when tx will become stable
        """
        pass

    @abstractmethod
    def burn_tokens_for_hex_address(self, recipient_hex: str, amount: int, payment_key: str) -> (bool, str, int):
        """Initiate transaction from main chain to partner chain, aka passive flow.

        Arguments:
            recipient_hex {str} -- recipient's hex address
            amount {int} -- amount of tokens to burn on the main chain
            payment_key {str} -- main chain address' signing key

        Returns:
            (bool, string, int) - True/False, transaction hash, and main chain block when tx will become stable
        """
        pass

    @abstractmethod
    def get_incoming_txs(self) -> dict:
        """
        Retrieves a list of incoming pending txs

        Returns:
            (dict) - JSON
        Raises:
            PartnerChainRpcException
        """
        pass

    @abstractmethod
    def get_mc_stable_block_for_incoming_tx(self, txHash: str) -> int:
        """
        Finds the stable block number on mainchain where an incoming tx is going to be finalized

        Arguments:
            Hash of incoming transaction to query for

        Returns:
            (int) - The mainchain block number on which the tx's balance will be available on partner chain
        """
        pass

    @abstractmethod
    def get_epoch_committee(self, epoch: int) -> PartnerChainRpcResponse:
        """
        Retrieves the committee for given epoch

        Arguments:
            epoch {int} -- partner chain epoch

        Returns:
            JSON dict {PartnerChainRpcResponse}
        """
        pass

    @abstractmethod
    def get_epoch_signatures(self, epoch: int) -> PartnerChainRpcResponse:
        """Returns the data required for committee handover and Merkle root insertion: partner chain parameters,
        next committee, Merkle root of outgoing transactions, and corresponding signatures.

        Arguments:
            epoch {int} -- partner chain epoch

        Returns:
            str -- JSON with keys 'params', 'committeeHandover', and 'outgoingTransactions'
        """
        pass

    @abstractmethod
    def get_ariadne_parameters(self, mc_epoch) -> PartnerChainRpcResponse:
        """Returns the configuration data for ariadne: d-parameter, permissioned candidates, trustless candidates

        Arguments:
            epoch {int} -- mainchain epoch

        Returns:
            str -- JSON with keys 'dParameter', 'permissionedCandidates', and 'candidateRegistrations'
        """
        pass

    @abstractmethod
    def get_trustless_candidates(self, mc_epoch: int, valid_only: bool) -> dict:
        """Retrieves all registered trustless candidates for given mc epoch.

        Arguments:
            mc_epoch {int}
            valid_only {bool} -- if True returns only valid registrations for an SPO.

        Return: A dict of SPOs. Example response:
        ```
        {
            "SPOPubKey1": [
                {"sidechainPubKey": "0x000000", "mainchainPubKey": "0x111111", "isValid": true, ...}
            ],
            "SPOPubKey2": [
                {"sidechainPubKey": "0x000001", "mainchainPubKey": "0x222222", "isValid": false, ...}
            ]
            "SPOPubKey3": [
                {"sidechainPubKey": "0x000002", "mainchainPubKey": "0x333333", "isValid": true, ...},
                {"sidechainPubKey": "0x000003", "mainchainPubKey": "0x444444", "isValid": true, ...}
            ]
        }
        ```
        """

    @abstractmethod
    def get_trustless_rotation_candidates(self, mc_epoch) -> dict:
        """Get trustless rotation candidates for a given MC epoch.
        Rotation candidates are set in config file. We pick them and check their statuses
        on the main chain to determine if they are active or inactive.

        Arguments:
            mc_epoch {int} -- MC epoch for which we want to get rotation candidates

        Returns:
            dict -- {"name:" <node_name>, "public_key": <public_key>, "status": "active"|"inactive"}
        """

    @abstractmethod
    def get_permissioned_candidates(self, mc_epoch: int, valid_only: bool) -> list:
        """Retrieves all permissioned candidates for given mc epoch.

        Arguments:
            mc_epoch {int}
            valid_only {bool} -- if True, returns only valid candidates

        Return: A list of candidates. Example response:
        ```
        [
            {"sidechainPublicKey": "0x000000", "auraPublicKey": "0x111111", "isValid": true, ...},
            {"sidechainPublicKey": "0x000001", "auraPublicKey": "0x222222", "isValid": false, ...}
        ]
        ```
        """

    @abstractmethod
    def get_permissioned_rotation_candidates(self, mc_epoch) -> dict:
        """Get permissioned rotation candidates for a given MC epoch.
        Rotation candidates are set in config file. We pick them and check their statuses
        on the main chain to determine if they are active or inactive.

        Arguments:
            mc_epoch {int} -- MC epoch for which we want to get rotation candidates

        Returns:
            dict -- {"name:" <node_name>, "public_key": <public_key>, "status": "active"|"inactive"}
        """

    @abstractmethod
    def get_committee_seats(self, mc_epoch=None) -> int:
        """Returns committee seats.

        Arguments:
            mc_epoch {int} -- main chain epoch, if omitted uses the current one.

        Returns:
            int -- committee seats for given mc epoch.
        """

    @abstractmethod
    def get_d_param(self, mc_epoch=None) -> DParam:
        """Returns d-param.

        Keyword Arguments:
            mc_epoch {int} -- main chain epoch, if omitted uses the current one. (default: {None})

        Returns:
            DParam -- number of permissioned and trustless candidates
        """

    @abstractmethod
    def get_registrations(self, mc_epoch, mc_key) -> PartnerChainRpcResponse:
        """Returns registration for a mc_key.

        Keyword Arguments:
            mc_epoch {int} -- main chain epoch, if omitted uses the current one
            mc_key {str} -- main chain public key

        Returns:
            dict - registration data
        """

    @abstractmethod
    def claim_tokens(self, mc_private_key_file, combined_proof, distributed_set_utxo=None) -> bool:
        """
        Claims tokens that have been locked on the partner chain, using a combined proof and an (optional)
        distributed set utxo, for faster lookup.

        Arguments: the private key file that is associated with the recipient of the lock transaction
        and the combined proof of that lock transaction, after the committee has signed it.

        Returns: boolean
        """
        pass

    @abstractmethod
    def get_outgoing_txs(self, epoch) -> dict:  # TODO: Verify dict or list
        """
        Retrieves a list of outgoing txs that happened in a sc epoch

        Arguments: the partner chain epoch

        Returns:
            (dict) - JSON
        """
        pass

    @abstractmethod
    def get_outgoing_tx_merkle_proof(self, epoch, txId) -> str:  # TODO: verify str or dict
        """
        Retrieves the merkle proof of an outgoing tx

        Arguments: the partner chain epoch and the index of the tx

        Returns:
            (str) - the merkle proof (TODO: or a dict?)
        """
        pass

    @abstractmethod
    def get_expected_tx_fees(self, wallet_type, tx_type) -> int:
        """
        Get the expected fee amount for a specific tx from the configuration

        Arguments: wallet type and transaction type

        Returns:
            (int) - The amount of tokens necessary for the fee of a tx
        """
        pass

    @abstractmethod
    def cardano_address_to_bech32(self, mc_address: str) -> str:
        """
        Convert a cardano address to bech32 format

        Arguments: Cardano address (string)

        Returns:
            (string) - The bech32 format of the address (hexstring)
        """
        pass

    @abstractmethod
    def check_epoch_signatures_uploaded(self, pc_epoch) -> bool:
        pass

    @abstractmethod
    def get_block_extrinsic_value(self, extrinsic_name: str, block_no: int) -> str:
        """
        Gets the value of an extrinsic from a block.

        Arguments: Extrinisic name (str), Block number (int)

        Returns:
            (string) - The value of that extrinsic in that block
        """
        pass

    @abstractmethod
    def extract_block_extrinsic_value(self, extrinsic_name: str, block: dict) -> str:
        """
        Extracts the value of an extrinsic from a block.

        Arguments: Extrinisic name (str), Block (dict)

        Returns:
            (string) - The value of that extrinsic in that block
        """
        pass

    @abstractmethod
    def get_block_header(self, block_no: int) -> str:
        """
        Gets the header of a block.

        Arguments: Block number (int)

        Returns:
            (string) - The header of that block
        """
        pass

    @abstractmethod
    def get_block(self, block_no: int) -> str:
        """
        Gets the whole block.

        Arguments: Block number (int)

        Returns:
            (string) - The block
        """
        pass

    @abstractmethod
    def get_validator_set(self, block) -> str:
        """Gets validator set for a given block.

        Arguments:
            block -- block object

        Returns:
            str -- block author public key
        """
        pass

    @abstractmethod
    def get_block_author(self, block, validator_set) -> str:
        """Gets the author of a block.

        Arguments:
            block -- block object
            validator_set -- validator set for given pc epoch

        Returns:
            str -- block author public key
        """
        pass

    @abstractmethod
    def get_mc_hash_from_pc_block_header(self, block) -> str:
        """
        Extracts the main chain hash from a partner chain block header.

        Arguments: Block (dict)

        Returns:
            (string) - The main chain hash associated with a block
        """
        pass

    @abstractmethod
    def get_mc_block_by_block_hash(self, block_hash):
        """
        Get main chain block by block hash

        Arguments: Block hash

        Returns:
            (dict) - The block
        """
        pass

    @abstractmethod
    def get_mc_block_by_timestamp(self, timestamp):
        """
        Get main chain block by timestamp

        Arguments: timestamp

        Returns:
            (dict) - The block
        """

    @abstractmethod
    def sign_address_association(self, address: str, stake_signing_key: str) -> AddressAssociationSignature:
        """
        Creates a signature of the association between a PC address and a Cardano address. This association along
        with the signature can be submitted to the network via :func:`submit_address_association` method to allow
        ADA delegators to participate in PC block production rewards.

        Arguments:
            address {str} -- PC address (hex format) to be associated with the Cardano address
            stake_signing_key {str} -- Cardano Stake Signing key in hex format

        Returns:
            AddressAssociationSignature
        """
        pass

    @abstractmethod
    def submit_address_association(self, signature: AddressAssociationSignature, wallet: Wallet) -> Transaction:
        """
        Submits the association between a PC address and a Cardano address to the network. This allows ADA delegators
        to participate in PC block production rewards.

        Arguments:
            signature {AddressAssociationSignature} -- Signature of the association
            wallet {Wallet} -- Wallet used to sign the transaction

        Returns:
            Transaction
        """
        pass

    @abstractmethod
    def get_address_association(self, stake_key_hash: str) -> str:
        """
        Retrieves the PC address associated with the Cardano address.

        Arguments:
            stake_key_hash {str} -- Stake verification key hash

        Returns:
            str -- PC SS58 address associated with the Cardano address
        """
        pass

    @abstractmethod
    def get_block_production_log(self, block_hash):
        """
        Retrieves block production log for latest block.

        Arguments:
            block_hash {str} -- PC block hash

        Returns:
            block production log
        """
        pass
