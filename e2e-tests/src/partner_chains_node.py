from .run_command import RunnerFactory
from config.api_config import ApiConfig, Node
import json
import re
import logging as logger


class PartnerChainsNodeException(Exception):
    def __init__(self, message="Partner Chain CLI error occurred", status_code=None):
        self.message = message
        self.status_code = status_code
        super().__init__(self.message)


class RegistrationSignatures:
    spo_public_key: str
    spo_signature: str
    sidechain_public_keys: str
    sidechain_signature: str

    def __init__(
        self,
        spo_public_key,
        spo_signature,
        sidechain_public_keys,
        sidechain_signature,
    ):
        self.spo_public_key = spo_public_key
        self.spo_signature = spo_signature
        self.sidechain_public_keys = sidechain_public_keys
        self.sidechain_signature = sidechain_signature


class PartnerChainsNode:
    def __init__(self, config: ApiConfig):
        self.config = config
        cli_config = config.stack_config.tools["partner_chains_node"]
        self.cli = cli_config.cli
        self.run_command = RunnerFactory.get_runner(cli_config.ssh, cli_config.shell)

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
            logger.error(f"Could not parse response of generate-signatures cmd: {result}")
            raise e
        return signatures

    def update_d_param(self, permissioned_candidates_count, registered_candidates_count, payment_key):
        update_d_param_cmd = (
            f"{self.cli} smart-contracts upsert-d-parameter "
            f"--genesis-utxo {self.config.genesis_utxo} "
            f"--permissioned-candidates-count {permissioned_candidates_count} "
            f"--registered-candidates-count {registered_candidates_count} "
            f"--payment-key-file {payment_key} "
            f"--ogmios-url {self.config.stack_config.ogmios_url} "
        )

        result = self.run_command.run(update_d_param_cmd)
        response = self.handle_response(result)
        tx_id = self.extract_transaction_id(response)

        if tx_id:
            return tx_id
        else:
            logger.error(f"Wrong response format of upsert-d-parameter command: {response}")
            return None

    def register_candidate(self, signatures: RegistrationSignatures, payment_key, spo_public_key, registration_utxo):
        register_cmd = (
            f"{self.cli} smart-contracts register "
            f"--payment-key-file {payment_key} "
            f"--genesis-utxo {self.config.genesis_utxo} "
            f"--spo-public-key {spo_public_key} "
            f"--sidechain-public-keys {signatures.sidechain_public_keys} "
            f"--spo-signature {signatures.spo_signature} "
            f"--sidechain-signature {signatures.sidechain_signature} "
            f"--registration-utxo {registration_utxo} "
            f"--ogmios-url {self.config.stack_config.ogmios_url} "
        )

        result = self.run_command.run(register_cmd, timeout=self.config.timeouts.register_cmd)
        response = self.handle_response(result)
        tx_id = self.extract_transaction_id(response)

        if tx_id:
            return tx_id
        else:
            logger.error(f"Wrong response format of register command: {response}")
            return None

    def deregister_candidate(self, payment_key, spo_public_key):
        deregister_cmd = (
            f"{self.cli} smart-contracts deregister "
            f"--payment-key-file {payment_key} "
            f"--genesis-utxo {self.config.genesis_utxo} "
            f"--spo-public-key {spo_public_key} "
            f"--ogmios-url {self.config.stack_config.ogmios_url} "
        )

        result = self.run_command.run(deregister_cmd, timeout=self.config.timeouts.deregister_cmd)
        response = self.handle_response(result)
        tx_id = self.extract_transaction_id(response)

        if tx_id:
            return tx_id
        else:
            logger.error(f"Wrong response format from deregister command: {response}")
            return None

    def upsert_permissioned_candidates(self, governance_key, new_candidates_list: dict[str, Node]):
        # Create permissioned candidates file to be used in CLI command
        candidates_file_content = "\n".join(
            f"{candidate.public_key}:{candidate.aura_public_key}:{candidate.grandpa_public_key}"
            for candidate in new_candidates_list.values()
        )
        permissioned_candidates_file = "/tmp/permissioned_candidates.csv"
        save_file_cmd = f"echo '{candidates_file_content}' > {permissioned_candidates_file}"
        self.run_command.run(save_file_cmd)

        update_candidates_cmd = (
            f"{self.cli} smart-contracts upsert-permissioned-candidates "
            f"--payment-key-file {governance_key} "
            f"--genesis-utxo {self.config.genesis_utxo} "
            f"--permissioned-candidates-file {permissioned_candidates_file} "
            f"--ogmios-url {self.config.stack_config.ogmios_url} "
        )

        result = self.run_command.run(update_candidates_cmd, timeout=self.config.timeouts.register_cmd)
        response = self.handle_response(result)
        tx_id = self.extract_transaction_id(response)

        if tx_id:
            return tx_id
        else:
            logger.error(f"Wrong response format from upsert-permissioned-candidates command: {response}")
            return None

    def handle_response(self, result):
        if result.stderr and not result.stdout:
            logger.error(f"Error during command: {result.stderr}")
            raise PartnerChainsNodeException(result.stderr)

        return result.stdout

    def extract_transaction_id(self, log_output):
        pattern = r"Transaction output \'([a-f0-9]{64})\'"
        match = re.search(pattern, log_output)
        if match:
            return match.group(1)
        else:
            return None
