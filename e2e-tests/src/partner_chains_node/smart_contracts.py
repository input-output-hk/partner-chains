from dataclasses import dataclass
import logging
import re
import json
from config.api_config import ApiConfig, Node
from .models import RegistrationSignatures
from ..run_command import Runner, Result


@dataclass
class SmartContractsResponse:
    stdout: str
    stderr: str
    transaction_id: str = None
    json: dict = None


def parse_transaction_id(stdout: str) -> str:
    pattern = r"Transaction output \'([a-f0-9]{64})\'"
    match = re.search(pattern, stdout)
    if match:
        return match.group(1)
    else:
        return None


def parse_response(result: Result) -> SmartContractsResponse:
    response = SmartContractsResponse(stdout=result.stdout, stderr=result.stderr)
    response.transaction_id = parse_transaction_id(result.stdout)
    return response


def parse_json_response(result: Result) -> SmartContractsResponse:
    response = SmartContractsResponse(stdout=result.stdout, stderr=result.stderr)
    response.json = json.loads(result.stdout)
    return response


class SmartContracts:
    def __init__(self, cli, run_command: Runner, config: ApiConfig):
        self.cli = cli
        self.run_command = run_command
        self.config = config
        self.reserve = SmartContracts.Reserve(cli, run_command, config)

    def get_scripts(self):
        cmd = (
            f"{self.cli} smart-contracts get-scripts "
            f"--genesis-utxo {self.config.genesis_utxo} "
            f"--ogmios-url {self.config.stack_config.ogmios_url}"
        )
        response = self.run_command.run(cmd)
        return parse_json_response(response)

    def update_d_param(self, permissioned_candidates_count, registered_candidates_count, payment_key):
        cmd = (
            f"{self.cli} smart-contracts upsert-d-parameter "
            f"--genesis-utxo {self.config.genesis_utxo} "
            f"--permissioned-candidates-count {permissioned_candidates_count} "
            f"--registered-candidates-count {registered_candidates_count} "
            f"--payment-key-file {payment_key} "
            f"--ogmios-url {self.config.stack_config.ogmios_url} "
        )

        response = self.run_command.run(cmd)
        return parse_response(response)

    def register(self, signatures: RegistrationSignatures, payment_key, spo_public_key, registration_utxo):
        cmd = (
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

        response = self.run_command.run(cmd, timeout=self.config.timeouts.register_cmd)
        return parse_response(response)

    def deregister(self, payment_key, spo_public_key):
        cmd = (
            f"{self.cli} smart-contracts deregister "
            f"--payment-key-file {payment_key} "
            f"--genesis-utxo {self.config.genesis_utxo} "
            f"--spo-public-key {spo_public_key} "
            f"--ogmios-url {self.config.stack_config.ogmios_url} "
        )

        response = self.run_command.run(cmd, timeout=self.config.timeouts.deregister_cmd)
        return parse_response(response)

    def upsert_permissioned_candidates(self, governance_key, new_candidates_list: dict[str, Node]):
        logging.debug("Creating permissioned candidates file...")
        candidates_file_content = "\n".join(
            f"{candidate.public_key}:{candidate.aura_public_key}:{candidate.grandpa_public_key}:{candidate.imonline_public_key}"
            for candidate in new_candidates_list.values()
        )
        permissioned_candidates_file = "/tmp/permissioned_candidates.csv"
        save_file_cmd = f"echo '{candidates_file_content}' > {permissioned_candidates_file}"
        self.run_command.run(save_file_cmd)

        cmd = (
            f"{self.cli} smart-contracts upsert-permissioned-candidates "
            f"--payment-key-file {governance_key} "
            f"--genesis-utxo {self.config.genesis_utxo} "
            f"--permissioned-candidates-file {permissioned_candidates_file} "
            f"--ogmios-url {self.config.stack_config.ogmios_url} "
        )

        response = self.run_command.run(cmd, timeout=self.config.timeouts.register_cmd)
        return parse_response(response)

    class Reserve:
        def __init__(self, cli, run_command, config: ApiConfig):
            self.cli = cli
            self.run_command = run_command
            self.config = config

        def init(self, payment_key):
            cmd = (
                f"{self.cli} smart-contracts reserve init "
                f"--payment-key-file {payment_key} "
                f"--genesis-utxo {self.config.genesis_utxo} "
                f"--ogmios-url {self.config.stack_config.ogmios_url}"
            )
            response = self.run_command.run(cmd)
            return parse_response(response)

        def create(self, v_function_hash, initial_deposit, token, payment_key):
            cmd = (
                f"{self.cli} smart-contracts reserve create "
                f"--total-accrued-function-script-hash {v_function_hash} "
                f"--initial-deposit-amount {initial_deposit} "
                f"--token {token} "
                f"--payment-key-file {payment_key} "
                f"--genesis-utxo {self.config.genesis_utxo} "
                f"--ogmios-url {self.config.stack_config.ogmios_url}"
            )
            response = self.run_command.run(cmd)
            return parse_response(response)

        def release(self, reference_utxo, amount, payment_key):
            cmd = (
                f"{self.cli} smart-contracts reserve release "
                f"--reference-utxo {reference_utxo} "
                f"--amount {amount} "
                f"--payment-key-file {payment_key} "
                f"--genesis-utxo {self.config.genesis_utxo} "
                f"--ogmios-url {self.config.stack_config.ogmios_url}"
            )
            response = self.run_command.run(cmd)
            return parse_response(response)

        def deposit(self, amount, payment_key):
            cmd = (
                f"{self.cli} smart-contracts reserve deposit "
                f"--amount {amount} "
                f"--payment-key-file {payment_key} "
                f"--genesis-utxo {self.config.genesis_utxo} "
                f"--ogmios-url {self.config.stack_config.ogmios_url}"
            )
            response = self.run_command.run(cmd)
            return parse_response(response)

        def update_settings(self, v_function_hash, payment_key):
            cmd = (
                f"{self.cli} smart-contracts reserve update-settings "
                f"--total-accrued-function-script-hash {v_function_hash} "
                f"--payment-key-file {payment_key} "
                f"--genesis-utxo {self.config.genesis_utxo} "
                f"--ogmios-url {self.config.stack_config.ogmios_url}"
            )
            response = self.run_command.run(cmd)
            return parse_response(response)

        def handover(self, payment_key):
            cmd = (
                f"{self.cli} smart-contracts reserve handover "
                f"--payment-key-file {payment_key} "
                f"--genesis-utxo {self.config.genesis_utxo} "
                f"--ogmios-url {self.config.stack_config.ogmios_url}"
            )
            response = self.run_command.run(cmd)
            return parse_response(response)
