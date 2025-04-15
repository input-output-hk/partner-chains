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
        self.run_command = RunnerFactory.get_runner(cli_config.shell)
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

    def sign_block_producer_metadata(self, metadata, cross_chain_signing_key):
        cross_chain_signing_key = cross_chain_signing_key.to_string().hex()
        metadata_str = json.dumps(metadata)
        metadata_file_name = f"/tmp/metadata_{uuid.uuid4().hex}.json"
        save_file_cmd = f"echo '{metadata_str}' > {metadata_file_name}"
        self.run_command.run(save_file_cmd)

        sign_block_producer_metadata_cmd = (
            f"{self.cli} sign-block-producer-metadata "
            f"--genesis-utxo {self.config.genesis_utxo} "
            f"--metadata-file {metadata_file_name} "
            f"--cross-chain-signing-key {cross_chain_signing_key}"
        )

        result = self.run_command.run(sign_block_producer_metadata_cmd)
        if not result.stdout:
            logging.error(f"Command failed with stderr: {result.stderr}")
            raise PartnerChainsNodeException("Failed to sign block producer metadata")
            
        try:
            response = json.loads(result.stdout)
            return BlockProducerMetadataSignature(
                cross_chain_pub_key=response["cross_chain_pub_key"],
                cross_chain_pub_key_hash=response["cross_chain_pub_key_hash"],
                encoded_message=response["encoded_message"],
                encoded_metadata=response["encoded_metadata"],
                signature=response["signature"],
            )
        except json.JSONDecodeError as e:
            logging.error(f"Could not parse response of sign-block-producer-metadata cmd: {result.stdout}")
            raise PartnerChainsNodeException(f"Invalid JSON response: {e}")
        except KeyError as e:
            logging.error(f"Missing required field in response: {e}")
            raise PartnerChainsNodeException(f"Missing required field in response: {e}")
        except Exception as e:
            logging.error(f"Unexpected error in sign-block-producer-metadata: {e}")
            raise PartnerChainsNodeException(f"Unexpected error: {e}")

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
