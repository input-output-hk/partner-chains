from .blockchain_api import BlockchainApi, Transaction, Wallet
from .pc_epoch_calculator import PartnerChainEpochCalculator
from config.api_config import ApiConfig, Node
from substrateinterface import SubstrateInterface, Keypair, KeypairType
from eth_keys.datatypes import PrivateKey
from sqlalchemy import desc, select, func
from sqlalchemy.orm import Session
from sqlalchemy.exc import SQLAlchemyError
from src.db_sync.models import Tx, Block
from .decorators import long_running_function
import json
import hashlib
import logging as logger
from .run_command import RunnerFactory
from .cardano_cli import CardanoCli
from .sidechain_main_cli import SidechainMainCli
from .partner_chain_rpc import PartnerChainRpc, PartnerChainRpcResponse, PartnerChainRpcException, DParam
import string
import time
from scalecodec.base import RuntimeConfiguration


def _keypair_type_to_name(type):
    match type:
        case KeypairType.SR25519:
            return "SR25519"
        case KeypairType.ED25519:
            return "ED25519"
        case _:
            return "ECDSA"


def _keypair_name_to_type(type_name):
    match type_name:
        case 'SR25519':
            return KeypairType.SR25519
        case 'ED25519':
            return KeypairType.ED25519
        case _:
            return KeypairType.ECDSA


def is_hex(s):
    s = s[2:] if s.startswith('0x') else s
    try:
        int(s, 16)
        return True
    except ValueError:
        return False


class PermissionedCandidate:
    def __init__(self, config: Node):
        self.public_key = config.public_key
        self.aura_public_key = config.aura_public_key
        self.grandpa_public_key = config.grandpa_public_key


class SubstrateApi(BlockchainApi):
    def __init__(self, config: ApiConfig, secrets, db_sync: Session):
        self.config = config
        self.secrets = secrets
        self.db_sync = db_sync
        self.url = config.nodes_config.node.url
        self._substrate = None
        self.run_command = RunnerFactory.get_runner(config.stack_config.ssh, config.stack_config.tools_shell)
        self.cardano_cli = CardanoCli(config.main_chain, config.stack_config.tools["cardano_cli"])
        self.sidechain_main_cli = SidechainMainCli(config, self.cardano_cli)
        self.partner_chain_rpc = PartnerChainRpc(config.nodes_config.node.rpc_url)
        self.partner_chain_epoch_calculator = PartnerChainEpochCalculator(config)
        self.compact_encoder = RuntimeConfiguration().create_scale_object('Compact')
        with open("src/runtime_api.json") as file:
            self.custom_type_registry = json.load(file)

    @property
    def substrate(self):
        if self._substrate is None:
            self._substrate = SubstrateInterface(url=self.url, type_registry=self.custom_type_registry)
        return self._substrate

    def close(self):
        if self._substrate:
            self.substrate.close()
            self._substrate = None

    def get_latest_pc_block_number(self):
        block = self.substrate.get_block()
        logger.debug(f"Current partner chain block: {block}")
        return block["header"]["number"]

    def get_latest_mc_block_number(self):
        block = self.cardano_cli.get_block()
        logger.debug(f"Current main chain block: {block}")
        return block

    def get_pc_balance(self, address):
        balance = self.substrate.query("System", "Account", [address])["data"]["free"]
        logger.debug(f"SC address {address} balance: {balance}")
        return balance.value

    def get_mc_balance(self, address, policy_id="ADA"):
        tokensDict = self.cardano_cli.get_token_list_from_address(address)
        balance = 0
        if policy_id in tokensDict:
            balance = tokensDict[policy_id]
        logger.debug(f"MC address {address} balance: {balance} {policy_id}")
        return balance

    def get_outgoing_transactions(self, epoch):
        outgoing_txs = self.partner_chain_rpc.partner_chain_get_outgoing_transactions(epoch).result['transactions']
        logger.debug(f"Epoch {epoch} outgoing_txs: {outgoing_txs}")
        return outgoing_txs

    def build_transaction(self, tx: Transaction):
        tx._unsigned = self.substrate.compose_call(
            call_module="Balances",
            call_function="transfer_allow_death",
            call_params={
                "dest": tx.recipient,
                "value": tx.value,
            },
        )
        logger.debug(f"Transaction built {tx._unsigned}")
        return tx

    def lock_transaction(self, tx: Transaction):
        mc_address = tx.recipient
        tx.recipient = self.cardano_address_to_bech32(mc_address)
        if not tx.recipient or not all(c in string.hexdigits for c in tx.recipient[2:]):
            raise ValueError(f"Bech32 conversion of {mc_address} not successful: {tx.recipient}")

        tx._unsigned = self.substrate.compose_call(
            call_module="ActiveFlow", call_function="lock", call_params={"amount": tx.value, "receiver": tx.recipient}
        )
        logger.info(f"***********LOCK TX: {tx}*****************")
        logger.debug(f"Transaction built {tx._unsigned}")
        return tx

    def __create_signed_ecdsa_extrinsic(
        self,
        call,
        keypair,
        nonce: int = None,
        era: dict = None,
        tip: int = 0,
        tip_asset_id: int = None,
    ):
        """This function overrides default implementation of
        substrate.create_signed_extrinsic() with ecdsa algorithm,
        which is using keccak (ethereum) hashing lib, while we need to use blake.
        """
        self.substrate.init_runtime()

        # Retrieve nonce
        if nonce is None:
            nonce = self.substrate.get_account_nonce(keypair.ss58_address) or 0

        # Process era
        if era is None:
            era = "00"
        else:
            if isinstance(era, dict) and "current" not in era and "phase" not in era:
                # Retrieve current block id
                era["current"] = self.substrate.get_block_number(self.substrate.get_chain_finalised_head())

        # Sign payload
        signature_payload = self.substrate.generate_signature_payload(
            call=call, era=era, nonce=nonce, tip=tip, tip_asset_id=tip_asset_id
        )
        signature_payload_bytes = bytes(signature_payload.data)
        signer = PrivateKey(keypair.private_key)
        blake2b = hashlib.blake2b(digest_size=32)
        blake2b.update(signature_payload_bytes)
        signature = signer.sign_msg_hash(blake2b.digest()).to_bytes()
        signature_version = keypair.crypto_type

        # Create extrinsic
        extrinsic = self.substrate.runtime_config.create_scale_object(
            type_string="Extrinsic", metadata=self.substrate.metadata
        )
        blake2b = hashlib.blake2b(digest_size=32)
        blake2b.update(keypair.public_key)
        account_id = blake2b.hexdigest()

        value = {
            "account_id": f"0x{account_id}",
            "signature": f"0x{signature.hex()}",
            "call_function": call.value["call_function"],
            "call_module": call.value["call_module"],
            "call_args": call.value["call_args"],
            "nonce": nonce,
            "era": era,
            "tip": tip,
            "asset_id": {"tip": tip, "asset_id": tip_asset_id},
        }

        # Check if signature is MultiSignature, otherwise omit signature_version
        signature_cls = self.substrate.runtime_config.get_decoder_class("ExtrinsicSignature")
        if issubclass(signature_cls, self.substrate.runtime_config.get_decoder_class("Enum")):
            value["signature_version"] = signature_version

        extrinsic.encode(value)

        return extrinsic

    def sign_transaction(self, tx: Transaction, wallet: Wallet):
        if wallet.crypto_type and wallet.crypto_type == KeypairType.ECDSA:
            tx._signed = self.__create_signed_ecdsa_extrinsic(call=tx._unsigned, keypair=wallet.raw)
        else:
            tx._signed = self.substrate.create_signed_extrinsic(call=tx._unsigned, keypair=wallet.raw)
        logger.info(f"Transaction signed {tx._signed}")
        return tx

    @long_running_function
    def submit_transaction(self, tx: Transaction, wait_for_finalization):
        tx._receipt = self.substrate.submit_extrinsic(tx._signed, wait_for_finalization)
        logger.debug(f"Transaction sent {tx._receipt.extrinsic}")
        tx.hash = tx._receipt.extrinsic_hash
        tx.total_fee_amount = tx._receipt.total_fee_amount
        return tx

    def new_wallet(self):
        mnemonic = Keypair.generate_mnemonic()
        keypair = Keypair.create_from_mnemonic(mnemonic)
        keypair.crypto_type = KeypairType.SR25519
        wallet = Wallet()
        wallet.raw = keypair
        wallet.address = keypair.ss58_address
        wallet.private_key = keypair.private_key
        wallet.mnemonic = mnemonic
        wallet.seed = keypair.seed_hex
        wallet.public_key = keypair.public_key
        wallet.crypto_type = keypair.crypto_type
        logger.debug(f"New wallet created {wallet.address}")
        return wallet

    def get_wallet(self, address=None, public_key=None, secret=None, scheme=None):
        if not address:
            address = self.secrets["wallets"]["faucet-0"]["address"]
        if not public_key:
            public_key = self.secrets["wallets"]["faucet-0"]["public_key"]
        if not secret:
            secret = self.secrets["wallets"]["faucet-0"]["secret_seed"]
        if not scheme:
            scheme = self.secrets["wallets"]["faucet-0"]["scheme"]
        scheme_type = _keypair_name_to_type(scheme)

        if secret.startswith("//"):
            keypair = Keypair.create_from_uri(secret)
        else:
            keypair = Keypair(
                crypto_type=scheme_type, ss58_format=42, private_key=secret, seed_hex=bytes.fromhex(secret)
            )

        keypair.ss58_address = address
        keypair.public_key = bytes.fromhex(public_key)
        wallet = Wallet()
        wallet.raw = keypair
        wallet.address = keypair.ss58_address
        wallet.private_key = keypair.private_key
        wallet.crypto_type = keypair.crypto_type
        wallet.public_key = keypair.public_key
        wallet.seed = keypair.seed_hex
        return wallet

    def get_authorities(self):
        response = self.substrate.runtime_call("AuraApi", "authorities")
        logger.debug(f"Aura authorities {response}")
        return response.value

    #########

    def _read_json_file(self, filepath):
        with open(filepath, "r") as file:
            content = json.load(file)
        return content

    def _read_cardano_key_file(self, filepath):
        key_content = self._read_json_file(filepath)
        try:
            key = key_content["cborHex"][4:]  # Remove 5820 from cborHex string
        except Exception as e:
            logger.error(f"Could not parse cardano key file: {e}")
        return key.strip()

    def update_d_param(self, permissioned_candidates_count, registered_candidates_count):
        signing_key = self.config.nodes_config.governance_authority.mainchain_key

        result = self.sidechain_main_cli.update_d_param(
            permissioned_candidates_count,
            registered_candidates_count,
            signing_key,
        )

        if result:
            logger.info(
                f"Update of D Param of P: {permissioned_candidates_count} and R: {registered_candidates_count} "
                f" was successful and will take effect in 2 epochs "
            )
            return True, result
        else:
            return False, None

    #################
    def register_candidate(self, candidate_name):
        keys_files = self.config.nodes_config.nodes[candidate_name].keys_files
        # Get a UTxO from payment account
        utxos_json = self.cardano_cli.get_utxos(self.config.nodes_config.nodes[candidate_name].cardano_payment_addr)
        registration_utxo = next(filter(lambda utxo: utxos_json[utxo]["value"]["lovelace"] > 2500000, utxos_json), None)
        assert registration_utxo is not None, "ERROR: Could not find a well funded utxo for registration"

        signatures = self.sidechain_main_cli.get_signatures(
            registration_utxo,
            self._read_cardano_key_file(keys_files.spo_signing_key),
            self._read_json_file(keys_files.partner_chain_signing_key)['skey'],
            self.config.nodes_config.nodes[candidate_name].aura_public_key,
            self.config.nodes_config.nodes[candidate_name].grandpa_public_key,
        )

        txId, next_status_epoch = self.sidechain_main_cli.register_candidate(
            signatures,
            keys_files.cardano_payment_key,
            self._read_cardano_key_file(keys_files.spo_public_key),
            registration_utxo,
        )

        if txId and next_status_epoch:
            logger.info(
                f"Registration of {candidate_name} was successful and will take effect in "
                f"{next_status_epoch} MC epoch. Transaction id: {txId}"
            )
            return True, next_status_epoch
        else:
            return False, None

    def deregister_candidate(self, candidate_name):
        keys_files = self.config.nodes_config.nodes[candidate_name].keys_files
        txId, next_status_epoch = self.sidechain_main_cli.deregister_candidate(
            keys_files.cardano_payment_key,
            self._read_cardano_key_file(keys_files.spo_public_key),
        )

        if txId and next_status_epoch:
            logger.info(
                f"Deregistration of {candidate_name} was successful and will take effect in "
                f"{next_status_epoch} MC epoch. Transaction id: {txId}"
            )
            return True, next_status_epoch
        else:
            return False, None

    def get_pc_epoch(self):
        return self.partner_chain_rpc.partner_chain_get_status().result['sidechain']['epoch']

    def get_pc_epoch_phase(self, slot_number=None):
        return self.partner_chain_rpc.partner_chain_get_epoch_phase(slot_number).result['epochPhase']

    def get_params(self):
        return self.partner_chain_rpc.partner_chain_get_params().result

    def get_mc_epoch(self):
        return self.cardano_cli.get_epoch()

    def get_mc_slot(self):
        return self.cardano_cli.get_slot()

    def get_mc_block(self):
        return self.cardano_cli.get_block()

    def get_mc_sync_progress(self):
        return self.cardano_cli.get_sync_progress()

    def burn_tokens(self, recipient, amount, payment_key):
        assert self.substrate.is_valid_ss58_address(recipient), f"{recipient} is not a valid SS58 address"
        recipient_hex = self.address_to_hex(recipient)
        return self.burn_tokens_for_hex_address(recipient_hex, amount, payment_key)

    def burn_tokens_for_hex_address(self, recipient_hex, amount, payment_key):
        txHash = self.sidechain_main_cli.burn_tokens(recipient_hex, amount, payment_key)
        if txHash:
            tx_block_no = self.get_mc_block_no_by_tx_hash(txHash)
            mc_stable_block = tx_block_no + self.config.main_chain.security_param
            logger.info(
                f"Burn tx of {amount} tokens to {recipient_hex} was successful, "
                f"and will become stable at mc block {mc_stable_block}. Transaction id: {txHash}"
            )
            return True, txHash, mc_stable_block
        else:
            return False, None, None

    def address_to_hex(self, address):
        return self.substrate.ss58_decode(address)

    def wait_for_next_pc_block(self):
        logger.info('Waiting for next partner chain block')
        old_block = self.get_latest_pc_block_number()
        i = 0
        success = True
        latest_block = old_block
        while latest_block == old_block:
            time.sleep(2)
            latest_block = self.get_latest_pc_block_number()
            if i == 30:  # No block in 1 minute
                success = False
                break
            i = i + 1
        return success

    def wait_for_next_mc_block(self):
        logger.info('Waiting for next main chain block')
        old_block = self.cardano_cli.get_block()
        latest_block = old_block
        i = 0
        success = True
        while latest_block == old_block:
            time.sleep(10)
            latest_block = self.cardano_cli.get_block()
            if i == 24:  # No block in 4 minutes
                success = False
                break
            i = i + 1
        return success

    def get_incoming_txs(self) -> dict:
        response = self.partner_chain_rpc.partner_chain_get_incoming_transactions()
        if response.error:
            raise PartnerChainRpcException(f"Couldn't get incoming txs: {response.error.message}", response.error.code)
        return response.result

    def get_mc_stable_block_for_incoming_tx(self, txHash) -> int:
        pendingTxs = self.get_incoming_txs()
        tx_stable_at_mc_block = 0
        for pendingTx in pendingTxs['awaitingMcStability']:
            if pendingTx['txHash'] == txHash:
                tx_stable_at_mc_block = pendingTx['stableAtMainchainBlock']
        assert tx_stable_at_mc_block != 0, f"ERROR: Burn tx not identified as pending: {pendingTxs}"
        return tx_stable_at_mc_block

    def get_epoch_committee(self, epoch) -> PartnerChainRpcResponse:
        logger.info(f"Getting committee for epoch {epoch}")
        response = self.partner_chain_rpc.partner_chain_get_epoch_committee(epoch)
        if response.error:
            logger.error(f"Couldn't get committee for epoch {epoch}: {response.error}")
        return response

    def get_epoch_signatures(self, epoch) -> PartnerChainRpcResponse:
        logger.info(f"Getting signatures for epoch {epoch}")
        response = self.partner_chain_rpc.partner_chain_get_epoch_signatures(epoch)
        if response.error:
            logger.error(f"Couldn't get signatures for epoch {epoch}: {response.error}")
        return response

    def claim_tokens(self, mc_private_key_file, combined_proof, distributed_set_utxo=None) -> bool:
        return self.sidechain_main_cli.claim_tokens(
            mc_private_key_file, combined_proof, distributed_set_utxo=distributed_set_utxo
        )

    def get_outgoing_txs(self, epoch) -> dict:
        return self.partner_chain_rpc.partner_chain_get_outgoing_transactions(epoch)

    def get_outgoing_tx_merkle_proof(self, epoch, txId) -> str:
        return self.partner_chain_rpc.partner_chain_get_outgoing_transaction_merkle_proof(epoch, txId).result

    def get_expected_tx_fees(self, wallet_type, tx_type):
        wallet_type_name = _keypair_type_to_name(wallet_type)
        return eval(f"self.config.nodes_config.fees.{wallet_type_name}.{tx_type}")

    def get_status(self):
        return self.partner_chain_rpc.partner_chain_get_status().result

    def get_trustless_candidates(self, mc_epoch, valid_only):
        logger.info(f"Getting trustless candidates for {mc_epoch} MC epoch.")
        registrations = self.partner_chain_rpc.partner_chain_get_ariadne_parameters(mc_epoch).result[
            "candidateRegistrations"
        ]
        if valid_only:
            registrations = {
                spo: [candidate for candidate in candidates if candidate["isValid"]]
                for spo, candidates in registrations.items()
                if any(candidate["isValid"] for candidate in candidates)
            }
        return registrations

    def get_trustless_rotation_candidates(self, mc_epoch):
        logger.info(f"Getting trustless rotation candidates for {mc_epoch} MC epoch.")

        # get rotation candidates from config
        rotation_candidates = [
            {"name": name, "public_key": node.public_key, "status": "inactive"}
            for name, node in self.config.nodes_config.nodes.items()
            if node.rotation_candidate
        ]

        if not rotation_candidates:
            logger.warning("No trustless rotation candidates found in config")
            return None

        # get candidates from chain
        try:
            registrations = self.get_trustless_candidates(mc_epoch, valid_only=True)
        except (KeyError, TypeError) as e:
            logger.error(f"Couldn't get trustless candidates: {e}")
            return None

        # update status of rotation candidates
        for candidates in registrations.values():
            for candidate in candidates:
                rotation_candidate = next(
                    (
                        rotation_candidate
                        for rotation_candidate in rotation_candidates
                        if rotation_candidate["public_key"] == candidate["sidechainPubKey"]
                    ),
                    None,
                )
                if rotation_candidate:
                    rotation_candidate["status"] = "active"

        return rotation_candidates

    def get_permissioned_candidates(self, mc_epoch, valid_only):
        logger.info(f"Getting permissioned candidates for {mc_epoch} MC epoch.")
        candidates = self.partner_chain_rpc.partner_chain_get_ariadne_parameters(mc_epoch).result[
            "permissionedCandidates"
        ]
        if valid_only:
            candidates = [candidate for candidate in candidates if candidate["isValid"]]
        return candidates

    def get_permissioned_rotation_candidates(self, mc_epoch):
        logger.info(f"Getting permissioned rotation candidates for {mc_epoch} MC epoch.")
        # get rotation candidates from config
        rotation_candidates = [
            {"name": name, "public_key": node.public_key, "status": "inactive"}
            for name, node in self.config.nodes_config.nodes.items()
            if node.permissioned_candidate
        ]

        if not rotation_candidates:
            logger.warning("No permissioned rotation candidates found in config")
            return None

        # get candidates from chain
        try:
            registrations = self.get_permissioned_candidates(mc_epoch, valid_only=True)
        except (KeyError, TypeError) as e:
            logger.error(f"Couldn't get permissioned candidates: {e}")
            return None

        # update status of rotation candidates
        for candidate in registrations:
            rotation_candidate = next(
                (
                    rotation_candidate
                    for rotation_candidate in rotation_candidates
                    if rotation_candidate["public_key"] == candidate["sidechainPublicKey"]
                ),
                None,
            )
            if rotation_candidate:
                rotation_candidate["status"] = "active"

        return rotation_candidates

    def get_ariadne_parameters(self, mc_epoch):
        logger.info(f"Getting ariadne parameters for {mc_epoch} MC epoch.")
        return self.partner_chain_rpc.partner_chain_get_ariadne_parameters(mc_epoch).result

    def get_registrations(self, mc_epoch, mc_key):
        logger.info(f"Getting registrations for {mc_epoch} MC epoch and {mc_key} MC key.")
        return self.partner_chain_rpc.partner_chain_get_registrations(mc_epoch=mc_epoch, mc_key=mc_key).result

    def get_committee_seats(self, mc_epoch=None):
        if not mc_epoch:
            mc_epoch = self.get_mc_epoch()
        d_param = self.get_d_param(mc_epoch)
        return d_param.permissioned_candidates_number + d_param.trustless_candidates_number

    def get_d_param(self, mc_epoch=None) -> DParam:
        if not mc_epoch:
            mc_epoch = self.get_mc_epoch()
        response = self.partner_chain_rpc.partner_chain_get_ariadne_parameters(mc_epoch).result["dParameter"]
        d_param = DParam(response["numPermissionedCandidates"], response["numRegisteredCandidates"])
        return d_param

    def cardano_address_to_bech32(self, mc_address: str):
        bech32_config = self.config.stack_config.tools["bech32"]
        bech32_addr = self.run_command.run(f"{bech32_config.cli} <<< {mc_address}")
        if not bech32_addr.stdout or bech32_addr.stderr:
            raise Exception(bech32_addr.stderr)
        return '0x' + bech32_addr.stdout.strip()

    def check_epoch_signatures_uploaded(self, pc_epoch=None):
        signatures = self.partner_chain_rpc.partner_chain_get_signatures_to_upload().result
        if not signatures:
            return True
        if not pc_epoch:
            return False  # We don't have a sc epoch to wait for, so wait until all epochs are relayed
        for signature in signatures:
            if signature["epoch"] == pc_epoch and signature["merkleRoots"] != []:
                return False  # Wait until merkleRoots is empty or all epochs relayed
        return True

    def add_permissioned_candidate(self, candidate_name: str):
        candidate = PermissionedCandidate(self.config.nodes_config.nodes[candidate_name])
        txId, next_status_epoch = self.sidechain_main_cli.update_permissioned_candidates(
            self.config.nodes_config.governance_authority.mainchain_key, [candidate], []
        )

        if txId and next_status_epoch:
            logger.info(
                f"Addition of permissioned candidate {candidate_name} was successful and will take effect in MC epoch "
                f"{next_status_epoch}. Transaction id: {txId}"
            )
            return True, next_status_epoch
        else:
            return False, None

    def remove_permissioned_candidate(self, candidate_name: str):
        candidate = PermissionedCandidate(self.config.nodes_config.nodes[candidate_name])
        txId, next_status_epoch = self.sidechain_main_cli.update_permissioned_candidates(
            self.config.nodes_config.governance_authority.mainchain_key, [], [candidate]
        )

        if txId and next_status_epoch:
            logger.info(
                f"Removal of permissioned candidate {candidate_name} was successful and will take effect in "
                f"{next_status_epoch} MC epoch. Transaction id: {txId}"
            )
            return True, next_status_epoch
        else:
            return False, None

    def get_block_extrinsic_value(self, extrinsic_name, block_no):
        block = self.get_block(block_no)
        return self.extract_block_extrinsic_value(extrinsic_name, block)

    def extract_block_extrinsic_value(self, extrinsic_name, block):
        for extr in block["extrinsics"]:
            if extr["call"]["call_module"]["name"] == extrinsic_name:
                # Convert <class 'scalecodec.types.GenericExtrinsic'> to python dict
                extrinsic_dict = extr.value_serialized
                return extrinsic_dict["call"]["call_args"][0]["value"]
        return 0

    def get_block_header(self, block_no):
        return self.substrate.get_block_header(block_number=block_no)["header"]

    def get_block(self, block_no):
        block_hash = self.substrate.get_block_hash(block_no)
        return self.substrate.get_block(block_hash)

    def _block_header_encoder_and_signature_extractor(self, header: dict):
        signature = False
        header_encoded = bytes.fromhex(header["parentHash"][2:]).hex()
        # Convert block number to compact
        header["number"] = self.compact_encoder.encode(header["number"]).to_hex()
        header_encoded += bytes.fromhex(header["number"][2:]).hex()
        header_encoded += bytes.fromhex(header["stateRoot"][2:]).hex()
        header_encoded += bytes.fromhex(header["extrinsicsRoot"][2:]).hex()
        logs_encoded = ""
        consensus_cnt = 0
        consensus_encoded = ""
        for log in header["digest"]["logs"]:
            log = log.value_serialized
            if "Seal" in log.keys():
                # Do not include the signature in the encoded header.
                # We want to hash the header and sign to get this signature
                signature = log["Seal"][1]
            elif "PreRuntime" in log.keys():
                if is_hex(log["PreRuntime"][0]):
                    prefix = str(log["PreRuntime"][0])[2:]
                else:
                    logger.error(f"PreRuntime key is not hex: {log['PreRuntime'][0]}")
                    return None, None
                if is_hex(log["PreRuntime"][1]):
                    suffix = str(log["PreRuntime"][1])[2:]
                else:
                    suffix = str(log["PreRuntime"][1]).encode("utf-8").hex()
                suffix_length = str(hex(2 * len(suffix)))[2:]
                logs_encoded += "06" + prefix + suffix_length + suffix
            elif "Consensus" in log.keys():
                consensus_cnt += 1
                prefix = str(log["Consensus"][0])[2:]
                suffix = str(log["Consensus"][1])[2:]
                if "0100000000000000" in suffix:  # Grandpa committee keys
                    suffix_prepend = self.config.block_encoding_suffix_grandpa
                else:  # Aura committee keys
                    suffix_prepend = self.config.block_encoding_suffix_aura
                consensus_encoded += "04" + prefix + suffix_prepend + suffix
            # Keep adding key to decode as the are added to the block header
        if consensus_cnt == 0:
            logs_prefix = "08"
        elif consensus_cnt == 1:
            logs_prefix = "0c"
        elif consensus_cnt == 2:
            logs_prefix = "10"
        else:
            logger.debug("New block type detected with more than 2 consensus logs. Please update encoder")
            return False, False
        header_encoded += logs_prefix + logs_encoded + consensus_encoded
        return header_encoded, signature

    def extract_block_author(self, block, candidates_pub_keys):
        block_header = block["header"]
        scale_header, signature = self._block_header_encoder_and_signature_extractor(block_header)
        if not scale_header or not signature:
            raise Exception(f'Could not encode header of block {block_header["number"]}')
        header_hash = hashlib.blake2b(bytes.fromhex(scale_header), digest_size=32).hexdigest()

        for pub_key in candidates_pub_keys:
            keypair_public = Keypair(
                ss58_address=self.substrate.ss58_encode(pub_key),
                crypto_type=KeypairType.SR25519,  # For our substrate implementation SR25519 is block authorship type
            )
            is_author = keypair_public.verify(bytes.fromhex(header_hash), bytes.fromhex(signature[2:]))
            if is_author:
                return pub_key
        return None

    def get_mc_hash_from_pc_block_header(self, block):
        mc_hash_key = "0x6d637368"
        header = block["header"]
        for log in header["digest"]["logs"]:
            log = log.value_serialized
            if "PreRuntime" in log.keys() and log["PreRuntime"][0] == mc_hash_key:
                return log["PreRuntime"][1][2:]
        return None

    def get_mc_block_no_by_tx_hash(self, tx_hash, retries=5, delay=10):
        query = (
            select(Block.block_no)
            .join(Tx, Tx.block_id == Block.id)
            .where(Tx.hash == func.decode(tx_hash, 'hex'))
            .order_by(desc(Tx.id))
            .limit(1)
        )
        block_no = self.__get_data_from_db_sync(query, retries=5, delay=10)
        logger.debug(f"Block no for tx: {tx_hash} was found. It's block number is {block_no}")
        return block_no

    def get_mc_block_by_block_hash(self, block_hash, retries=5, delay=10):
        query = select(Block).where(Block.hash == f"\\x{block_hash}").order_by(desc(Block.id)).limit(1)
        block = self.__get_data_from_db_sync(query, retries=5, delay=10)
        logger.debug(f"Block for hash: {block_hash} was found. It's block number is {block}")
        return block

    def get_mc_block_by_timestamp(self, timestamp, retries=5, delay=10):
        from datetime import datetime, timezone

        time = datetime.fromtimestamp(timestamp, timezone.utc)

        query = select(Block).where(Block.time <= time).order_by(desc(Block.id)).limit(1)
        block = self.__get_data_from_db_sync(query, retries, delay)
        logger.debug(f"Block for timestamp: {timestamp} was found. It's block number is {block}")
        return block

    def __get_data_from_db_sync(self, query, retries=5, delay=10):
        for _ in range(retries):
            try:
                data = self.db_sync.scalar(query)
                if data is not None:
                    return data
                else:
                    logger.debug(f"Data was not found for query: {query}. Retrying")

            except SQLAlchemyError as e:
                logger.exception(f"Query: {query} failed with error {e}. Retrying")
                self.db_sync.rollback()

            # If the query was not successful, wait for a while before retrying
            time.sleep(delay)

        # If the query still fails after retrying, raise an exception
        logger.error(f"Query: {query} failed after {retries} retries")
        raise Exception(f"Query: {query} failed after {retries} retries")
