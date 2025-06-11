from .blockchain_api import BlockchainApi, Transaction, Wallet
from .pc_epoch_calculator import PartnerChainEpochCalculator
from config.api_config import ApiConfig
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
from .cardano_cli import CardanoCli
from .partner_chains_node.node import PartnerChainsNode
from .partner_chain_rpc import PartnerChainRpc, PartnerChainRpcResponse, DParam
import time
from scalecodec.base import ScaleBytes


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


class SubstrateApi(BlockchainApi):
    def __init__(self, config: ApiConfig, secrets, db_sync: Session):
        self.config = config
        self.secrets = secrets
        self.db_sync = db_sync
        self.url = config.nodes_config.node.url
        self._substrate = None
        self.cardano_cli = CardanoCli(config.main_chain, config.stack_config.tools["cardano_cli"])
        self.partner_chains_node = PartnerChainsNode(config)
        self.partner_chain_rpc = PartnerChainRpc(config.nodes_config.node.rpc_url)
        self.partner_chain_epoch_calculator = PartnerChainEpochCalculator(config)
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
        tx._receipt = self.substrate.submit_extrinsic(tx._signed, wait_for_finalization=wait_for_finalization)
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
        wallet.public_key = keypair.public_key.hex()
        wallet.crypto_type = keypair.crypto_type
        logger.debug(f"New wallet created {wallet.address}")
        return wallet

    def get_wallet(self, address, public_key, secret, scheme):
        scheme_type = _keypair_name_to_type(scheme)

        if secret.startswith("//") or secret.count(" ") >= 11:
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
        wallet.public_key = keypair.public_key.hex()
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

    #################

    def read_cardano_key_file(self, filepath) -> str:
        key_content = self._read_json_file(filepath)
        try:
            key = key_content["cborHex"][4:]  # Remove 5820 from cborHex string
        except Exception as e:
            logger.error(f"Could not parse cardano key file: {e}")
        return key.strip()

    def update_d_param(self, genesis_utxo, permissioned_candidates_count, registered_candidates_count):
        signing_key = self.config.nodes_config.governance_authority.mainchain_key

        response = self.partner_chains_node.smart_contracts.update_d_param(
            genesis_utxo,
            permissioned_candidates_count,
            registered_candidates_count, signing_key
        )
        tx_id = response.json["transaction_submitted"]
        effective_in_mc_epoch = self._effective_in_mc_epoch()

        if tx_id:
            logger.info(
                f"Update of D Param of P: {permissioned_candidates_count} and R: {registered_candidates_count} "
                f" was successful and will take effect in {effective_in_mc_epoch} epoch. Transaction id: {tx_id}"
            )
            return True, effective_in_mc_epoch
        else:
            logger.error(f"Update of D Param failed, STDOUT: {response.stdout}, STDERR: {response.stderr}")
            return False, None

    def upsert_permissioned_candidates(self, genesis_utxo, new_candidates_list):
        response = self.partner_chains_node.smart_contracts.upsert_permissioned_candidates(
        	genesis_utxo,
            self.config.nodes_config.governance_authority.mainchain_key,
            new_candidates_list
        )
        tx_id = response.json["transaction_submitted"]
        effective_in_mc_epoch = self._effective_in_mc_epoch()

        if tx_id:
            logger.info(
                f"Success! New permissioned candidates are set and will be effective in "
                f"{effective_in_mc_epoch} MC epoch. Transaction id: {tx_id}"
            )
            return True, effective_in_mc_epoch
        else:
            logger.error(f"Upsert permissioned candidates failed, STDOUT: {response.stdout}, STDERR: {response.stderr}")
            return False, None

    def register_candidate(self, genesis_utxo, candidate_name):
        keys_files = self.config.nodes_config.nodes[candidate_name].keys_files
        # Get a UTxO from payment account
        utxos_json = self.cardano_cli.get_utxos(self.config.nodes_config.nodes[candidate_name].cardano_payment_addr)
        registration_utxo = next(filter(lambda utxo: utxos_json[utxo]["value"]["lovelace"] > 2500000, utxos_json), None)
        assert registration_utxo is not None, "ERROR: Could not find a well funded utxo for registration"

        signatures = self.partner_chains_node.get_signatures(
            genesis_utxo,
            registration_utxo,
            self.read_cardano_key_file(keys_files.spo_signing_key),
            self._read_json_file(keys_files.partner_chain_signing_key)['skey'],
            self.config.nodes_config.nodes[candidate_name].aura_public_key,
            self.config.nodes_config.nodes[candidate_name].grandpa_public_key,
        )

        response = self.partner_chains_node.smart_contracts.register(
            genesis_utxo,
            signatures,
            keys_files.cardano_payment_key,
            self.read_cardano_key_file(keys_files.spo_public_key),
            registration_utxo,
        )
        tx_id = response.json["transaction_submitted"]
        effective_in_mc_epoch = self._effective_in_mc_epoch()

        if tx_id:
            logger.info(
                f"Registration of {candidate_name} was successful and will take effect in "
                f"{effective_in_mc_epoch} MC epoch. Transaction id: {tx_id}"
            )
            return True, effective_in_mc_epoch
        else:
            logger.error(
                f"Registration of {candidate_name} failed, STDOUT: {response.stdout}, STDERR: {response.stderr}"
            )
            return False, None

    def deregister_candidate(self, genesis_utxo, candidate_name):
        keys_files = self.config.nodes_config.nodes[candidate_name].keys_files
        response = self.partner_chains_node.smart_contracts.deregister(
            genesis_utxo,
            keys_files.cardano_payment_key,
            self.read_cardano_key_file(keys_files.spo_public_key)
        )
        tx_id = response.json["transaction_submitted"]
        effective_in_mc_epoch = self._effective_in_mc_epoch()

        if tx_id:
            logger.info(
                f"Deregistration of {candidate_name} was successful and will take effect in "
                f"{effective_in_mc_epoch} MC epoch. Transaction id: {tx_id}"
            )
            return True, effective_in_mc_epoch
        else:
            logger.error(
                f"Deregistration of {candidate_name} failed, STDOUT: {response.stdout}, STDERR: {response.stderr}"
            )
            return False, None

    def get_pc_epoch(self):
        return self.partner_chain_rpc.partner_chain_get_status().result['sidechain']['epoch']

    def get_pc_epoch_blocks(self, epoch):
        """Returns a range of blocks produced in the given epoch.
        The algorithm is as follows:
        1. Find any block in the given epoch.
            This task is crucial to find the range, especially when there are a lot of empty slots.
            It works as follows:
            - calculate the difference between the current epoch and the given epoch
            - use it to calculate the number of slots (blocks) to go back
            - check the epoch of the block
                * if it matches, exit loop
                * if it doesn't match, and the epoch diff > 1, reduce the number of slots to go back by one epoch
                * else, reduce the number of slots to go back by one slot
        2. Find the first block in the given epoch. Once we've found a block in the given epoch,
            we're iterating over each previous block until the epoch changes.
        3. Find the last block in the given epoch. Once we've found the first block, we go forward by one epoch,
            and iterate over each previous block until the epoch matches the searched epoch again.

        Args:
            epoch (int): epoch to search for

        Raises:
            ValueError: if the given epoch is greater than or equal to the current epoch

        Returns:
            range: range of blocks produced in the given epoch
        """
        current_block = self.get_latest_pc_block_number()
        current_pc_epoch = self.get_pc_epoch()
        if epoch >= current_pc_epoch:
            raise ValueError(
                f"Cannot get blocks for current or future epoch {epoch}. Current epoch is {current_pc_epoch}."
            )

        # search for a block in <epoch>
        slots_in_epoch = self.config.nodes_config.slots_in_epoch
        slots_to_go_back = (current_pc_epoch - epoch) * slots_in_epoch
        found_epoch = 0
        while found_epoch != epoch:
            block_in_searched_epoch = self.get_block(block_no=(current_block - slots_to_go_back))
            result = self.substrate.query(
                "SessionCommitteeManagement", "CurrentCommittee", block_hash=block_in_searched_epoch["header"]["hash"]
            )
            found_epoch = result.value["epoch"]
            if epoch - found_epoch > 1:
                slots_to_go_back -= slots_in_epoch
            else:
                slots_to_go_back -= 1
        logger.info(f"Found a block in epoch {epoch}: {block_in_searched_epoch['header']['number']}")

        # search for the first block in <epoch>
        while found_epoch == epoch:
            first_block = block_in_searched_epoch
            result = self.substrate.query(
                "SessionCommitteeManagement", "CurrentCommittee", block_hash=first_block["header"]["parentHash"]
            )
            found_epoch = result.value["epoch"]
            block_in_searched_epoch = self.get_block(block_no=first_block["header"]["number"] - 1)
        logger.info(f"Found the first block in epoch {epoch}: {first_block['header']['number']}")

        # search for the last block in <epoch>
        slots_to_go_forward = slots_in_epoch
        found_epoch = 0
        while found_epoch != epoch:
            last_block = self.get_block(block_no=(first_block["header"]["number"] + slots_to_go_forward))
            result = self.substrate.query(
                "SessionCommitteeManagement", "CurrentCommittee", block_hash=last_block["header"]["hash"]
            )
            found_epoch = result.value["epoch"]
            slots_to_go_forward -= 1
        logger.info(f"Found the last block in epoch {epoch}: {last_block['header']['number']}")

        return range(first_block["header"]["number"], last_block["header"]["number"] + 1)

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

    def get_epoch_committee(self, epoch) -> PartnerChainRpcResponse:
        logger.info(f"Getting committee for epoch {epoch}")
        response = self.partner_chain_rpc.partner_chain_get_epoch_committee(epoch)
        if response.error:
            logger.error(f"Couldn't get committee for epoch {epoch}: {response.error}")
        return response

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
        response = self.partner_chain_rpc.partner_chain_get_ariadne_parameters(mc_epoch)
        if response.error:
            logger.error(f"Couldn't get permissioned candidates for {mc_epoch} MC epoch: {response.error}")
            return None
        candidates = response.result["permissionedCandidates"]
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
        registrations = self.get_permissioned_candidates(mc_epoch, valid_only=True)
        if not registrations:
            logger.error("Couldn't get permissioned candidates")
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

    def get_block_extrinsic_value(self, extrinsic_name, block_no):
        block = self.get_block(block_no)
        return self.extract_block_extrinsic_value(extrinsic_name, block)

    def extract_block_extrinsic_value(self, extrinsic_name, block):
        for extrinsic in block["extrinsics"]:
            if extrinsic["call"]["call_module"]["name"] == extrinsic_name:
                # Convert <class 'scalecodec.types.GenericExtrinsic'> to python dict
                extrinsic_dict = extrinsic.value_serialized
                return extrinsic_dict["call"]["call_args"][0]["value"]
        return 0

    def get_block_header(self, block_no):
        return self.substrate.get_block_header(block_number=block_no)["header"]

    def get_block(self, block_no=None):
        return self.substrate.get_block(block_number=block_no)

    def get_validator_set(self, block):
        return self.substrate.query("Session", "ValidatorsAndKeys", block_hash=block["header"]["parentHash"])

    def get_block_author_and_slot(self, block, validator_set):
        """Custom implementation of substrate.get_block(include_author=True) to get block author, and block slot.
        py-substrate-interface does not work because it calls "Validators" function from "Session" pallet,
        which in our node is disabled and returns empty list. Here we use "ValidatorsAndKeys".
        The function then iterates over "PreRuntime" logs and once it finds aura engine, it gets the slot
        number and uses the result of modulo to get the author by index from the validator set.
        Note: py-substrate-interface was also breaking at this point because we have another "PreRuntime" log
        for mcsh engine (main chain hash) which is not supported by py-substrate-interface.
        """
        for log_data in block["header"]["digest"]["logs"]:
            engine = bytes(log_data[1][0])
            if "PreRuntime" in log_data and engine == b'aura':
                aura_predigest = self.substrate.runtime_config.create_scale_object(
                    type_string='RawAuraPreDigest', data=ScaleBytes(bytes(log_data[1][1]))
                )

                aura_predigest.decode(check_remaining=self.substrate.config.get("strict_scale_decode"))

                rank_validator = aura_predigest.value["slot_number"] % len(validator_set)

                block_author = validator_set[rank_validator]
                block["author"] = block_author.value[1]["aura"]
                block["slot"] = aura_predigest.value["slot_number"]
                break

        if "author" not in block:
            block_no = block["header"]["number"]
            logger.error(f"Could not find author for block {block_no}. No PreRuntime log found with aura engine.")
            return None
        return block["author"], block["slot"]

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
        block_no = self.__get_data_from_db_sync(query, retries=retries, delay=delay)
        logger.debug(f"Block no for tx: {tx_hash} was found. It's block number is {block_no}")
        return block_no

    def get_mc_block_by_block_hash(self, block_hash, retries=5, delay=10):
        query = select(Block).where(Block.hash == f"\\x{block_hash}").order_by(desc(Block.id)).limit(1)
        block = self.__get_data_from_db_sync(query, retries=retries, delay=delay)
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

    def _effective_in_mc_epoch(self):
        """Calculates main chain epoch in which smart contracts candidates related operation will be effective."""
        return self.cardano_cli.get_epoch() + 2

    def sign_address_association(self, genesis_utxo, address, stake_signing_key):
        return self.partner_chains_node.sign_address_association(genesis_utxo, address, stake_signing_key)

    def sign_block_producer_metadata(self, genesis_utxo, metadata, cross_chain_signing_key):
        return self.partner_chains_node.sign_block_producer_metadata(genesis_utxo, metadata, cross_chain_signing_key)

    @long_running_function
    def submit_address_association(self, signature, wallet):
        tx = Transaction()
        tx._unsigned = self.substrate.compose_call(
            call_module="AddressAssociations",
            call_function="associate_address",
            call_params={
                "partnerchain_address": signature.partner_chain_address,
                "signature": signature.signature,
                "stake_public_key": signature.stake_public_key,
            },
        )
        logger.debug(f"Transaction built {tx._unsigned}")

        if wallet.crypto_type and wallet.crypto_type == KeypairType.ECDSA:
            tx._signed = self.__create_signed_ecdsa_extrinsic(call=tx._unsigned, keypair=wallet.raw)
        else:
            tx._signed = self.substrate.create_signed_extrinsic(call=tx._unsigned, keypair=wallet.raw)
        logger.debug(f"Transaction signed {tx._signed}")

        tx._receipt = self.substrate.submit_extrinsic(tx._signed, wait_for_inclusion=True)
        logger.debug(f"Transaction sent {tx._receipt.extrinsic}")
        tx.hash = tx._receipt.extrinsic_hash
        tx.total_fee_amount = tx._receipt.total_fee_amount
        return tx

    @long_running_function
    def submit_block_producer_metadata(self, metadata, signature, wallet):
        tx = Transaction()
        tx._unsigned = self.substrate.compose_call(
            call_module="BlockProducerMetadata",
            call_function="upsert_metadata",
            call_params={
                "metadata": metadata,
                "signature": signature.signature,
                "cross_chain_pub_key": signature.cross_chain_pub_key,
            },
        )
        logger.debug(f"Transaction built {tx._unsigned}")

        if wallet.crypto_type and wallet.crypto_type == KeypairType.ECDSA:
            tx._signed = self.__create_signed_ecdsa_extrinsic(call=tx._unsigned, keypair=wallet.raw)
        else:
            tx._signed = self.substrate.create_signed_extrinsic(call=tx._unsigned, keypair=wallet.raw)
        logger.debug(f"Transaction signed {tx._signed}")

        tx._receipt = self.substrate.submit_extrinsic(tx._signed, wait_for_inclusion=True)
        logger.debug(f"Transaction sent {tx._receipt.extrinsic}")
        tx.hash = tx._receipt.extrinsic_hash
        tx.total_fee_amount = tx._receipt.total_fee_amount
        return tx

    def get_address_association(self, stake_key_hash):
        result = self.substrate.query("AddressAssociations", "AddressAssociations", [f"0x{stake_key_hash}"])
        logger.debug(f"Address association for {stake_key_hash}: {result}")
        return result.value

    def get_block_producer_metadata(self, cross_chain_public_key_hash: str):
        result = self.substrate.query(
            "BlockProducerMetadata", "BlockProducerMetadataStorage", [f"0x{cross_chain_public_key_hash}"]
        )
        logger.debug(f"Block producer metadata for {cross_chain_public_key_hash}: {result}")
        return result.value

    def get_block_production_log(self, block_hash=None):
        result = self.substrate.query("BlockProductionLog", "Log", block_hash=block_hash)
        logger.debug(f"Block production log: {result}")
        return result.value

    def get_block_participation_data(self, block_hash=None):
        result = self.substrate.query("TestHelperPallet", "LatestParticipationData", block_hash=block_hash)
        logger.debug(f"Block participation data: {result}")
        return result.value

    @long_running_function
    def set_block_producer_margin_fee(self, margin_fee, wallet):
        tx = Transaction()
        tx._unsigned = self.substrate.compose_call(
            call_module="BlockProducerFees", call_function="set_fee", call_params={"fee_numerator": margin_fee}
        )
        logger.debug(f"Transaction built {tx._unsigned}")

        if wallet.crypto_type and wallet.crypto_type == KeypairType.ECDSA:
            tx._signed = self.__create_signed_ecdsa_extrinsic(call=tx._unsigned, keypair=wallet.raw)
        else:
            tx._signed = self.substrate.create_signed_extrinsic(call=tx._unsigned, keypair=wallet.raw)
        logger.debug(f"Transaction signed {tx._signed}")

        tx._receipt = self.substrate.submit_extrinsic(tx._signed, wait_for_inclusion=True)
        logger.debug(f"Transaction sent {tx._receipt.extrinsic}")
        tx.hash = tx._receipt.extrinsic_hash
        tx.total_fee_amount = tx._receipt.total_fee_amount
        return tx

    def get_initial_pc_epoch(self):
        block = self.get_block()
        block_hash = block["header"]["hash"]
        session_index_result = self.substrate.query("Session", "CurrentIndex", block_hash=block_hash)
        epoch_result = self.substrate.query("Sidechain", "EpochNumber", block_hash=block_hash)
        logger.debug(f"Current session index: {session_index_result}, epoch number: {epoch_result}")
        initial_epoch = epoch_result.value - session_index_result.value
        return initial_epoch

    @long_running_function
    def set_governed_map_main_chain_scripts(self, address, policy_id, wallet):
        logger.info(f"Setting governed map address {address} with policy id {policy_id}")
        tx = Transaction()
        call = self.substrate.compose_call(
            call_module="GovernedMap",
            call_function="set_main_chain_scripts",
            call_params={"new_main_chain_script": {"validator_address": address, "asset_policy_id": policy_id}},
        )
        tx._unsigned = self.substrate.compose_call(call_module="Sudo", call_function="sudo", call_params={"call": call})
        logger.debug(f"Transaction built {tx._unsigned}")

        if wallet.crypto_type and wallet.crypto_type == KeypairType.ECDSA:
            tx._signed = self.__create_signed_ecdsa_extrinsic(call=tx._unsigned, keypair=wallet.raw)
        else:
            tx._signed = self.substrate.create_signed_extrinsic(call=tx._unsigned, keypair=wallet.raw)
        logger.debug(f"Transaction signed {tx._signed}")

        tx._receipt = self.substrate.submit_extrinsic(tx._signed, wait_for_inclusion=True)
        logger.debug(f"Transaction sent {tx._receipt.extrinsic}")
        tx.hash = tx._receipt.extrinsic_hash
        tx.total_fee_amount = tx._receipt.total_fee_amount
        return tx

    def get_governed_map(self):
        result = self.substrate.query_map("GovernedMap", "Mapping")
        governed_map = {}
        for key, value in result:
            governed_map[key.value] = value.value
            logger.debug(f"Key: {key.value}, Value: {value.value}")
        logger.debug(f"Governed map: {governed_map}")
        return governed_map

    def get_governed_map_key(self, key):
        result = self.substrate.query("GovernedMap", "Mapping", [key])
        logger.debug(f"Governed map for key {key}: {result}")
        return result.value

    def subscribe_governed_map_initialization(self):
        current_main_chain_block = self.get_mc_block()
        max_main_chain_block = current_main_chain_block + self.config.main_chain.security_param

        def subscription_handler(obj, update_nr, subscription_id):
            if update_nr == 0:
                logger.debug(f"Current initialization state: {obj}")
            if update_nr > 0 and obj:
                return True
            if self.get_mc_block() > max_main_chain_block:
                logger.warning("Max main chain block reached. Stopping subscription.")
                self.substrate.rpc_request("chain_unsubscribeNewHeads", [subscription_id])
                return False

        logger.info(
            f"Subscribing to Governed Map initialization. "
            f"Max main chain block: {max_main_chain_block} ({self.config.main_chain.security_param} blocks ahead)"
        )
        result = self.substrate.query("GovernedMap", "Initialized", subscription_handler=subscription_handler)
        return result

    def subscribe_governed_map_change(self, key=None, key_value=None):
        max_mc_reference_block = self.get_mc_block()

        def subscribed_change_handler(registered_changes):
            if key_value:
                return key_value if key_value in registered_changes else None
            elif key:
                return next((change for change in registered_changes if change[0] == key), None)
            elif registered_changes:
                return registered_changes
            else:
                return True  # e.g. governed map reinitialization with 0 changes

        def subscription_handler(obj, update_nr, subscription_id):
            block_no = obj["header"]["number"]
            logger.debug(f"New block #{block_no}")

            mc_hash = self.get_mc_hash_from_pc_block_header(obj)
            mc_block = self.get_mc_block_by_block_hash(mc_hash).block_no
            logger.debug(f"Main chain reference block: {mc_block}")

            subscribed_change = None
            block = self.substrate.get_block(block_number=block_no)
            for idx, extrinsic in enumerate(block["extrinsics"]):
                logger.debug(f"# {idx}: {extrinsic.value}")
                if (
                    extrinsic.value["call"]["call_module"] == "GovernedMap"
                    and extrinsic.value["call"]["call_function"] == "register_changes"
                ):
                    registered_changes = extrinsic.value["call"]["call_args"][0]["value"]
                    subscribed_change = subscribed_change_handler(registered_changes)
                    break
            if subscribed_change:
                self.substrate.rpc_request("chain_unsubscribeNewHeads", [subscription_id])
                return subscribed_change
            if mc_block > max_mc_reference_block:
                logger.warning("Max main chain block reached. Stopping subscription.")
                self.substrate.rpc_request("chain_unsubscribeNewHeads", [subscription_id])
                return False

        if key_value:
            change_to_observe_msg = f"Observing specific change: {key_value}"
        elif key:
            change_to_observe_msg = f"Observing changes for key: {key}"
        else:
            change_to_observe_msg = "Observing any changes"

        logger.info(
            f"Subscribing to Governed Map changes. {change_to_observe_msg}. "
            f"Max main chain reference block: {max_mc_reference_block}."
        )
        result = self.substrate.subscribe_block_headers(subscription_handler)
        return result

    def subscribe_token_transfer(self):
        max_mc_reference_block = self.get_mc_block()

        def subscription_handler(obj, update_nr, subscription_id):
            block_no = obj["header"]["number"]
            logger.debug(f"New block #{block_no}")

            mc_hash = self.get_mc_hash_from_pc_block_header(obj)
            mc_block = self.get_mc_block_by_block_hash(mc_hash).block_no
            logger.debug(f"Main chain reference block: {mc_block}")

            token_transfer_value = None
            block = self.substrate.get_block(block_number=block_no)
            for idx, extrinsic in enumerate(block["extrinsics"]):
                logger.debug(f"# {idx}: {extrinsic.value}")
                if (
                    extrinsic.value["call"]["call_module"] == "NativeTokenManagement"
                    and extrinsic.value["call"]["call_function"] == "transfer_tokens"
                ):
                    token_transfer_value = extrinsic.value["call"]["call_args"][0]["value"]
                    break
            if token_transfer_value:
                return token_transfer_value
            if mc_block > max_mc_reference_block:
                logger.warning("Max main chain block reached. Stopping subscription.")
                self.substrate.rpc_request("chain_unsubscribeNewHeads", [subscription_id])
                return False

        result = self.substrate.subscribe_block_headers(subscription_handler)
        return result
