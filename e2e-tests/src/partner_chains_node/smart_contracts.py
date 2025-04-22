import logging
import json
import uuid
from abc import ABC, abstractmethod
from dataclasses import dataclass
from config.api_config import ApiConfig, Node
from .models import RegistrationSignatures
from ..run_command import Runner, Result


@dataclass
class SmartContractsResponse:
    returncode: int
    stdout: str
    stderr: str
    json: dict = None


class SignatureHandler(ABC):
    @abstractmethod
    def handle_transaction(self, response: SmartContractsResponse, smart_contracts: "SmartContracts"):
        pass


class SingleSignatureHandler(SignatureHandler):
    def handle_transaction(self, response: SmartContractsResponse, smart_contracts: "SmartContracts"):
        # In the single-signature case, the transaction is already signed and submitted.
        # Simply return the response.
        return response


class MultiSignatureHandler(SignatureHandler):

    def sign_and_submit_tx(self, tx_cbor, smart_contracts: "SmartContracts"):
        witnesses = []
        for authority in smart_contracts.config.nodes_config.additional_governance_authorities:
            witness = smart_contracts.sign_tx(tx_cbor, authority.mainchain_key)
            witnesses.append(witness.json["cborHex"])
        submit_response = smart_contracts.assemble_and_submit_tx(tx_cbor, witnesses)
        return submit_response

    def handle_transaction(self, response: SmartContractsResponse, smart_contracts: "SmartContracts"):
        if isinstance(response.json, list):
            for tx in response.json:
                tx_cbor = tx["transaction_to_sign"]["tx"]["cborHex"]
                response = self.sign_and_submit_tx(tx_cbor, smart_contracts)
        else:
            tx_cbor = response.json["transaction_to_sign"]["tx"]["cborHex"]
            response = self.sign_and_submit_tx(tx_cbor, smart_contracts)

        return response


def handle_governance_signature(
    response: SmartContractsResponse, smart_contracts: "SmartContracts"
) -> SmartContractsResponse:
    def contains_key(data, key):
        if isinstance(data, dict):
            return key in data
        elif isinstance(data, list):
            return any(key in item for item in data if isinstance(item, dict))
        return False

    if contains_key(response.json, "transaction_to_sign"):
        handler = MultiSignatureHandler()
    else:
        handler = SingleSignatureHandler()

    return handler.handle_transaction(response, smart_contracts)


def parse_json_response(result: Result) -> SmartContractsResponse:
    response = SmartContractsResponse(returncode=result.returncode, stdout=result.stdout, stderr=result.stderr)
    try:
        response.json = json.loads(result.stdout)
    except json.JSONDecodeError as e:
        logging.warning(f"Failed to parse {result.stdout} as JSON. Error: {e}")
    return response


class SmartContracts:
    def __init__(self, cli, run_command: Runner, config: ApiConfig):
        self.cli = cli
        self.run_command = run_command
        self.config = config
        self.reserve = SmartContracts.Reserve(self)
        self.governance = SmartContracts.Governance(self)

    def get_scripts(self) -> dict:
        """Get all scripts from the node."""
        cmd = f"{self.cli} get-scripts"
        result = self.run_command.run(cmd)
        if not result.stdout:
            logging.error(f"Failed to get scripts: {result.stderr}")
            raise PartnerChainsNodeException(f"Failed to get scripts: {result.stderr}")
        try:
            response = parse_json_response(result)
            if not response or "addresses" not in response:
                logging.error(f"Invalid response format from get_scripts: {response}")
                raise PartnerChainsNodeException("Invalid response format from get_scripts")
            return response
        except Exception as e:
            logging.error(f"Error parsing get_scripts response: {e}")
            raise PartnerChainsNodeException(f"Error parsing get_scripts response: {e}")

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
        parsed_response = parse_json_response(response)
        return handle_governance_signature(parsed_response, self)

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
        return parse_json_response(response)

    def deregister(self, payment_key, spo_public_key):
        cmd = (
            f"{self.cli} smart-contracts deregister "
            f"--payment-key-file {payment_key} "
            f"--genesis-utxo {self.config.genesis_utxo} "
            f"--spo-public-key {spo_public_key} "
            f"--ogmios-url {self.config.stack_config.ogmios_url} "
        )

        response = self.run_command.run(cmd, timeout=self.config.timeouts.deregister_cmd)
        return parse_json_response(response)

    def upsert_permissioned_candidates(self, governance_key, new_candidates_list: dict[str, Node]):
        logging.debug("Creating permissioned candidates file...")
        candidates_file_content = "\n".join(
            f"{candidate.public_key}:{candidate.aura_public_key}:{candidate.grandpa_public_key}"
            for candidate in new_candidates_list.values()
        )
        permissioned_candidates_file = f"/tmp/permissioned_candidates_{uuid.uuid4().hex}.csv"
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
        parsed_response = parse_json_response(response)
        return handle_governance_signature(parsed_response, self)

    def sign_tx(self, transaction_cbor, payment_key):
        cmd = (
            f"{self.cli} smart-contracts sign-tx "
            f"--transaction {transaction_cbor} "
            f"--payment-key-file {payment_key} "
        )

        response = self.run_command.run(cmd)
        return parse_json_response(response)

    def assemble_and_submit_tx(self, transaction_cbor, witnesses):
        witnesses_str = " ".join(witnesses)
        cmd = (
            f"{self.cli} smart-contracts assemble-and-submit-tx "
            f"--transaction {transaction_cbor} "
            f"--witnesses {witnesses_str} "
            f"--ogmios-url {self.config.stack_config.ogmios_url} "
        )

        response = self.run_command.run(cmd)
        return parse_json_response(response)

    class Reserve:
        def __init__(self, parent: "SmartContracts"):
            self.cli = parent.cli
            self.run_command = parent.run_command
            self.config = parent.config
            self.parent = parent

        def init(self, payment_key):
            cmd = (
                f"{self.cli} smart-contracts reserve init "
                f"--payment-key-file {payment_key} "
                f"--genesis-utxo {self.config.genesis_utxo} "
                f"--ogmios-url {self.config.stack_config.ogmios_url}"
            )
            response = self.run_command.run(cmd)
            parsed_response = parse_json_response(response)
            return handle_governance_signature(parsed_response, self.parent)

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
            response = self.run_command.run(cmd, timeout=self.config.timeouts.main_chain_tx)
            parsed_response = parse_json_response(response)
            return handle_governance_signature(parsed_response, self.parent)

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
            return parse_json_response(response)

        def deposit(self, amount, payment_key):
            cmd = (
                f"{self.cli} smart-contracts reserve deposit "
                f"--amount {amount} "
                f"--payment-key-file {payment_key} "
                f"--genesis-utxo {self.config.genesis_utxo} "
                f"--ogmios-url {self.config.stack_config.ogmios_url}"
            )
            response = self.run_command.run(cmd)
            parsed_response = parse_json_response(response)
            return handle_governance_signature(parsed_response, self.parent)

        def update_settings(self, v_function_hash, payment_key):
            cmd = (
                f"{self.cli} smart-contracts reserve update-settings "
                f"--total-accrued-function-script-hash {v_function_hash} "
                f"--payment-key-file {payment_key} "
                f"--genesis-utxo {self.config.genesis_utxo} "
                f"--ogmios-url {self.config.stack_config.ogmios_url}"
            )
            response = self.run_command.run(cmd)
            parsed_response = parse_json_response(response)
            return handle_governance_signature(parsed_response, self.parent)

        def handover(self, payment_key):
            cmd = (
                f"{self.cli} smart-contracts reserve handover "
                f"--payment-key-file {payment_key} "
                f"--genesis-utxo {self.config.genesis_utxo} "
                f"--ogmios-url {self.config.stack_config.ogmios_url}"
            )
            response = self.run_command.run(cmd)
            parsed_response = parse_json_response(response)
            return handle_governance_signature(parsed_response, self.parent)

    class Governance:
        def __init__(self, parent: "SmartContracts"):
            self.cli = parent.cli
            self.run_command = parent.run_command
            self.config = parent.config
            self.parent = parent

        def update(self, payment_key, new_governance_authorities, new_governance_threshold=1):
            authorities_str = " ".join(new_governance_authorities)
            cmd = (
                f"{self.cli} smart-contracts governance update "
                f"--payment-key-file {payment_key} "
                f"--genesis-utxo {self.config.genesis_utxo} "
                f"--governance-authority {authorities_str} "
                f"--threshold {new_governance_threshold} "
                f"--ogmios-url {self.config.stack_config.ogmios_url}"
            )
            response = self.run_command.run(cmd)
            parsed_response = parse_json_response(response)
            return handle_governance_signature(parsed_response, self.parent)

        def get_policy(self):
            cmd = (
                f"{self.cli} smart-contracts governance get-policy "
                f"--genesis-utxo {self.config.genesis_utxo} "
                f"--ogmios-url {self.config.stack_config.ogmios_url}"
            )
            response = self.run_command.run(cmd)
            return parse_json_response(response)
