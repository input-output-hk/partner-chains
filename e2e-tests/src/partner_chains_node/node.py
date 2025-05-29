from src.run_command import RunnerFactory
from config.api_config import ApiConfig
from .smart_contracts import SmartContracts
from .models import AddressAssociationSignature, RegistrationSignatures, BlockProducerMetadataSignature
import json
import logging
import uuid


class PartnerChainsNodeException(Exception):
    def __init__(self, message="Partner Chain CLI error occurred", status_code=None):
        self.message = message
        self.status_code = status_code
        super().__init__(self.message)


class PartnerChainsNode:
    def __init__(self, config: ApiConfig):
        self.config = config
        cli_config = config.stack_config.tools["partner_chains_node"]
        self.cli = cli_config.cli
        self.run_command = RunnerFactory.get_runner(
            shell=cli_config.shell,
            pod=cli_config.pod,
            namespace=cli_config.namespace,
            container=cli_config.container
        )
        self.smart_contracts = SmartContracts(self.cli, self.run_command, config)

    def sign_address_association(self, partner_chain_address, stake_signing_key):
        sign_address_association_cmd = (
            f"{self.cli} sign-address-association "
            f"--genesis-utxo {self.config.genesis_utxo} "
            f"--partnerchain-address {partner_chain_address} "
            f"--signing-key {stake_signing_key}"
        )

        result = self.run_command.run(sign_address_association_cmd)
        try:
            response = json.loads(result.stdout)
            return AddressAssociationSignature(
                partner_chain_address=response["partnerchain_address"],
                signature=response["signature"],
                stake_public_key=response["stake_public_key"],
            )
        except Exception as e:
            logging.error(f"Could not parse response of sign-address-association cmd: {result}")
            raise e

    def sign_block_producer_metadata(self, stake_key_hash: str, payment_key: str) -> dict:
        # Remove '0x' prefix if present and ensure correct length
        if stake_key_hash.startswith('0x'):
            stake_key_hash = stake_key_hash[2:]
        if len(stake_key_hash) != 56:  # Expected length for stake key hash
            logging.error(f"Invalid stake key hash length: {len(stake_key_hash)}")
            raise PartnerChainsNodeException(f"Invalid stake key hash length: {len(stake_key_hash)}")

        cmd = (
            f"{self.cli} sign-block-producer-metadata "
            f"--stake-key-hash {stake_key_hash} "
            f"--payment-key {payment_key}"
        )
        result = self.run_command.run(cmd)
        if not result.stdout:
            logging.error(f"Failed to sign block producer metadata: {result.stderr}")
            raise PartnerChainsNodeException(f"Failed to sign block producer metadata: {result.stderr}")
        try:
            response = json.loads(result.stdout)
            if not response or "signature" not in response:
                logging.error(f"Invalid response format from sign_block_producer_metadata: {response}")
                raise PartnerChainsNodeException("Invalid response format from sign_block_producer_metadata")
            return response
        except Exception as e:
            logging.error(f"Error parsing sign_block_producer_metadata response: {e}")
            raise PartnerChainsNodeException(f"Error parsing sign_block_producer_metadata response: {e}")

    def get_signatures(
        self,
        sidechain_registration_utxo,
        spo_signing_key,
        sidechain_signing_key,
        aura_verification_key,
        grandpa_verification_key,
    ):
        get_signatures_cmd = (
            f"{self.cli} registration-signatures "
            f"--genesis-utxo {self.config.genesis_utxo} "
            f"--mainchain-signing-key {spo_signing_key} "
            f"--sidechain-signing-key {sidechain_signing_key} "
            f"--registration-utxo {sidechain_registration_utxo}"
        )

        result = self.run_command.run(get_signatures_cmd)

        try:
            registration_signatures = json.loads(result.stdout)

            spo_public_key = registration_signatures["spo_public_key"]
            spo_signature = registration_signatures["spo_signature"]
            sidechain_public_key = registration_signatures["sidechain_public_key"]
            sidechain_signature = registration_signatures["sidechain_signature"]

            signatures = RegistrationSignatures(
                spo_public_key=spo_public_key,
                spo_signature=spo_signature,
                sidechain_public_keys=f"{sidechain_public_key}:{aura_verification_key}:{grandpa_verification_key}",
                sidechain_signature=sidechain_signature,
            )
        except Exception as e:
            logging.error(f"Could not parse response of generate-signatures cmd: {result}")
            raise e
        return signatures
