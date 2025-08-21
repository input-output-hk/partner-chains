import json
from .run_command import RunnerFactory
from config.api_config import MainChainConfig, Tool
import logging as logger
import uuid
import hashlib
import ecdsa
from bech32 import bech32_encode, convertbits


def cbor_to_bech32(cbor: str, prefix: str) -> str:
    decoded = bytes.fromhex(cbor)[2:]
    converted = convertbits(decoded, 8, 5)  # Convert bytes to 5-bit groups
    bech32_address = bech32_encode(prefix, converted)
    return bech32_address


def hex_to_bech32(hex_string: str, prefix: str) -> str:
    if hex_string.startswith("0x"):
        hex_string = hex_string[2:]
    byte_array = bytes.fromhex(hex_string)
    converted = convertbits(byte_array, 8, 5)  # Convert bytes to 5-bit groups
    bech32_address = bech32_encode(prefix, converted)
    return bech32_address


class CardanoCli:
    def __init__(self, config: MainChainConfig, cardano_cli: Tool):
        self.cli = cardano_cli.path
        self.network = config.network
        self.run_command = RunnerFactory.get_runner(cardano_cli.runner)

    def query_tip(self) -> int:
        cmd = f"{self.cli} latest query tip {self.network}"
        result = self.run_command.exec(cmd)
        return json.loads(result.stdout)

    def get_epoch(self) -> int:
        return self.query_tip()["epoch"]

    def get_block(self) -> int:
        return self.query_tip()["block"]

    def get_slot(self) -> int:
        return self.query_tip()["slot"]

    def get_sync_progress(self) -> float:
        return self.query_tip()["syncProgress"]

    def get_utxos(self, addr):
        cmd = f"{self.cli} latest query utxo --address {addr} {self.network} --out-file /dev/stdout"
        result = self.run_command.exec(cmd)
        if result.stderr:
            logger.error(result.stderr)
        
        parsed_stdout = json.loads(result.stdout)
        
        # In newer cardano-cli versions, the UTXO list might be nested under "unspentUtxos"
        if isinstance(parsed_stdout, dict) and "unspentUtxos" in parsed_stdout:
            return parsed_stdout["unspentUtxos"]
        
        return parsed_stdout

    def get_token_list_from_address(self, address):
        logger.debug("Getting list of tokens and ADA with amounts...")
        utxosJson = self.get_utxos(address)
        tokensDict = {}
        for utxo in utxosJson.keys():
            for token_policy in utxosJson[utxo]["value"].keys():
                if token_policy == "lovelace":
                    if "ADA" in tokensDict.keys():
                        tokensDict["ADA"] += utxosJson[utxo]["value"][token_policy]
                    else:
                        tokensDict["ADA"] = utxosJson[utxo]["value"][token_policy]
                else:
                    for token_name in utxosJson[utxo]["value"][token_policy].keys():
                        if token_policy + "." + token_name in tokensDict.keys():
                            tokensDict[token_policy + "." + token_name] += utxosJson[utxo]["value"][token_policy][
                                token_name
                            ]
                        else:
                            tokensDict[token_policy + "." + token_name] = utxosJson[utxo]["value"][token_policy][
                                token_name
                            ]
        return tokensDict

    def get_stake_pool_id(self, cold_vkey, output_format="hex"):
        logger.debug("Getting Stake Pool Id...")
        cmd = f'{self.cli} latest stake-pool id --stake-pool-verification-key {cold_vkey} --output-format "{output_format}"'
        result = self.run_command.exec(cmd)
        if result.stderr:
            logger.error(result.stderr)
        pool_id = result.stdout.strip()
        logger.debug(f"Stake Pool Id: {pool_id}")
        return pool_id

    def get_stake_snapshot_of_pool(self, pool_id):
        logger.debug("Getting pool's stake distribution...")
        cmd = f"{self.cli} latest query stake-snapshot {self.network} --stake-pool-id {pool_id}"
        result = self.run_command.exec(cmd)
        if result.stderr:
            logger.error(result.stderr)
        return json.loads(result.stdout)

    def __parse_keys_from_stdout(self, stdout):
        valid_json_string = "[" + stdout.replace("\r", "").replace("}\n{", "},\n{") + "]"
        skey_vkey_pair = json.loads(valid_json_string)
        return skey_vkey_pair[0], skey_vkey_pair[1]

    def generate_payment_keys(self):
        logger.debug("Generating payment keys...")
        cmd = f"{self.cli} latest address key-gen --verification-key-file /dev/stdout --signing-key-file /dev/stdout"
        result = self.run_command.exec(cmd)
        if result.stderr:
            logger.error(result.stderr)
            return None, None

        signing_key, verification_key = self.__parse_keys_from_stdout(result.stdout)
        logger.debug(f"Payment signing key: {signing_key}")
        logger.debug(f"Payment verification key: {verification_key}")
        return signing_key, verification_key

    def generate_stake_keys(self):
        logger.debug("Generating stake keys...")
        cmd = (
            f"{self.cli} latest stake-address key-gen "
            "--verification-key-file /dev/stdout --signing-key-file /dev/stdout"
        )
        result = self.run_command.exec(cmd)
        if result.stderr:
            logger.error(result.stderr)
            return None, None

        signing_key, verification_key = self.__parse_keys_from_stdout(result.stdout)
        logger.debug(f"Stake signing key: {signing_key}")
        logger.debug(f"Stake verification key: {verification_key}")
        return signing_key, verification_key

    def build_address(self, payment_vkey):
        logger.debug("Building address...")
        cmd = f"{self.cli} latest address build --payment-verification-key {payment_vkey} {self.network}"
        result = self.run_command.exec(cmd)
        if result.stderr:
            logger.error(result.stderr)
        return result.stdout.strip()

    def generate_cross_chain_keys(self):
        logger.debug("Generating cross chain keys...")
        pkey = ecdsa.SigningKey.generate(ecdsa.SECP256k1)

        pkey_hex = pkey.to_string().hex()
        vkey_bytes = pkey.get_verifying_key().to_string("compressed")
        vkey_hex = vkey_bytes.hex()

        blake2b = hashlib.blake2b(digest_size=28)
        blake2b.update(vkey_bytes)
        vkey_hash = blake2b.digest().hex()

        logger.debug(f"Cross chain signing key: {pkey_hex}")
        logger.debug(f"Cross chain verification key: {vkey_hex}")
        logger.debug(f"Cross chain verification key hash: {vkey_hash}")

        return pkey, vkey_hex, vkey_hash

    def get_stake_key_hash(self, stake_key):
        logger.debug("Getting stake key hash...")
        cmd = f"{self.cli} latest stake-address key-hash --stake-verification-key {stake_key}"
        result = self.run_command.exec(cmd)
        if result.stderr:
            logger.error(result.stderr)
        return result.stdout.strip()

    def get_address_key_hash(self, payment_vkey):
        logger.debug("Getting address key hash...")
        cmd = f"{self.cli} latest address key-hash --payment-verification-key {payment_vkey}"
        result = self.run_command.exec(cmd)
        if result.stderr:
            logger.error(result.stderr)
        return result.stdout.strip()

    def get_policy_id(self, script_file):
        logger.debug("Calculating policy id...")
        cmd = f"{self.cli} latest transaction policyid --script-file {script_file}"
        result = self.run_command.exec(cmd)
        if result.stderr:
            logger.error(result.stderr)
        return result.stdout.strip()

    def build_mint_tx(self, tx_in, address, lovelace, amount, asset_id, policy_script_filepath):
        logger.debug("Building transaction for minting tokens...")
        minting_token_tx_filepath = f"/tmp/minting_tx_{uuid.uuid4().hex}.raw"
        cmd = (
            f"{self.cli} latest transaction build "
            f"--tx-in {tx_in} "
            f"--tx-out '{address}+{lovelace}+{amount} {asset_id}' "
            f"--change-address {address} "
            f"--mint='{amount} {asset_id}' "
            f"--minting-script-file {policy_script_filepath} "
            f"--out-file {minting_token_tx_filepath} "
            f"{self.network}"
        )
        result = self.run_command.exec(cmd)
        if result.stderr:
            logger.error(result.stderr)
        return result.stdout.strip(), minting_token_tx_filepath

    def build_tx_with_reference_script(self, tx_in, address, lovelace, reference_script_file, change_address):
        logger.debug("Building transaction with reference script...")
        raw_tx_filepath = f"/tmp/reference_script_tx_{uuid.uuid4().hex}.raw"
        cmd = (
            f"{self.cli} latest transaction build "
            f"--tx-in {tx_in} "
            f"--tx-out '{address}+{lovelace}' "
            f"--tx-out-reference-script-file {reference_script_file} "
            f"--change-address {change_address} "
            f"--out-file {raw_tx_filepath} "
            f"{self.network}"
        )
        result = self.run_command.exec(cmd)
        if result.stderr:
            logger.error(result.stderr)
        return result.stdout.strip(), raw_tx_filepath

    def sign_transaction(self, tx_filepath, signing_key):
        logger.debug("Signing transaction...")
        signed_tx_filepath = f"/tmp/signed_tx_{uuid.uuid4().hex}.signed"
        cmd = (
            f"{self.cli} latest transaction sign "
            f"--tx-body-file {tx_filepath} "
            f"--signing-key-file {signing_key} "
            f"--out-file {signed_tx_filepath} "
            f"{self.network}"
        )
        result = self.run_command.exec(cmd)
        if result.stderr:
            logger.error(result.stderr)
        return signed_tx_filepath

    def submit_transaction(self, tx_filepath):
        logger.debug("Submitting transaction...")
        cmd = f"{self.cli} latest transaction submit --tx-file {tx_filepath} {self.network}"
        result = self.run_command.exec(cmd)
        if result.stderr:
            logger.error(result.stderr)
            return result.stderr
        try:
            # New cardano-cli versions may output JSON with txhash
            return json.loads(result.stdout)
        except json.JSONDecodeError:
            # Fallback for older versions
            return result.stdout
