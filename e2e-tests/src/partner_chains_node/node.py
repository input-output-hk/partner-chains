from src.run_command import RunnerFactory
from config.api_config import ApiConfig
from .smart_contracts import SmartContracts
from .models import AddressAssociationSignature, RegistrationSignatures, BlockProducerMetadataSignature
import json
import logging


class PartnerChainsNodeException(Exception):
    def __init__(self, message="Partner Chain CLI error occurred", status_code=None):
        self.message = message
        self.status_code = status_code
        super().__init__(self.message)


class PartnerChainsNode:
    def __init__(self, config: ApiConfig):
        self.config = config
        cli_config = config.stack_config.tools.node
        self.cli = cli_config.path
        self.run_command = RunnerFactory.get_runner(cli_config.runner)
        self.smart_contracts = SmartContracts(self.cli, self.run_command, config)

    def sign_address_association(self, genesis_utxo: str, partner_chain_address, stake_signing_key):
        sign_address_association_cmd = (
            f"{self.cli} sign-address-association "
            f"--genesis-utxo {genesis_utxo} "
            f"--partnerchain-address {partner_chain_address} "
            f"--signing-key {stake_signing_key}"
        )

        result = self.run_command.exec(sign_address_association_cmd)
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

    def sign_block_producer_metadata(self, genesis_utxo, metadata_file, cross_chain_signing_key):
        cross_chain_signing_key = cross_chain_signing_key.to_string().hex()

        sign_block_producer_metadata_cmd = (
            f"{self.cli} sign-block-producer-metadata "
            f"upsert "
            f"--genesis-utxo {genesis_utxo} "
            f"--metadata-file {metadata_file} "
            f"--cross-chain-signing-key {cross_chain_signing_key}"
        )

        result = self.run_command.exec(sign_block_producer_metadata_cmd)
        try:
            response = json.loads(result.stdout)

            return BlockProducerMetadataSignature(
                cross_chain_pub_key=response["cross_chain_pub_key"],
                cross_chain_pub_key_hash=response["cross_chain_pub_key_hash"],
                encoded_message=response["encoded_message"],
                encoded_metadata=response["encoded_metadata"],
                signature=response["signature"],
            )
        except Exception as e:
            logging.error(f"Could not parse response of sign-block-producer-metadata cmd: {result}")
            raise e

    def get_signatures(
        self,
        genesis_utxo: str,
        sidechain_registration_utxo,
        spo_signing_key,
        sidechain_signing_key,
        aura_verification_key,
        grandpa_verification_key,
    ):
        get_signatures_cmd = (
            f"{self.cli} registration-signatures "
            f"--genesis-utxo {genesis_utxo} "
            f"--mainchain-signing-key {spo_signing_key} "
            f"--sidechain-signing-key {sidechain_signing_key} "
            f"--registration-utxo {sidechain_registration_utxo}"
        )

        result = self.run_command.exec(get_signatures_cmd)

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
