from src.run_command import RunnerFactory
from config.api_config import ApiConfig
from .smart_contracts import SmartContracts
from .models import AddressAssociationSignature, RegistrationSignatures
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
        cli_config = config.stack_config.tools["partner_chains_node"]
        self.cli = cli_config.cli
        self.run_command = RunnerFactory.get_runner(cli_config.ssh, cli_config.shell)
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
                stake_public_key=response["stake_public_key"]
            )
        except Exception as e:
            logging.error(f"Could not parse response of sign-address-association cmd: {result}")
            raise e

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
