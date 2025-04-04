import json
from .run_command import RunnerFactory
from config.api_config import MainChainConfig, Tool
import logging as logger
import hashlib
import ecdsa


class CardanoCli:
    def __init__(self, config: MainChainConfig, cardano_cli: Tool):
        self.cli = cardano_cli.cli
        self.network = config.network
        self.run_command = RunnerFactory.get_runner(cardano_cli.ssh, cardano_cli.shell)

    def query_tip(self) -> int:
        cmd = f"{self.cli} latest query tip {self.network}"
        result = self.run_command.run(cmd)
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
        result = self.run_command.run(cmd)
        if result.stderr:
            logger.error(result.stderr)
        return json.loads(result.stdout)

    def get_token_list_from_address(self, address):
        logger.info("Getting list of tokens and ADA with amounts...")
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
        logger.info("Getting Stake Pool Id")
        cmd = f'{self.cli} latest stake-pool id --stake-pool-verification-key {cold_vkey} --output-format "{output_format}"'
        result = self.run_command.run(cmd)
        if result.stderr:
            logger.error(result.stderr)
        pool_id = result.stdout.strip()
        logger.info(f"Stake Pool Id: {pool_id}")
        return pool_id

    def get_stake_snapshot_of_pool(self, pool_id):
        logger.info("Getting pool's stake distribution")
        cmd = f"{self.cli} latest query stake-snapshot {self.network} --stake-pool-id {pool_id}"
        result = self.run_command.run(cmd)
        if result.stderr:
            logger.error(result.stderr)
        return json.loads(result.stdout)

    def generate_stake_keys(self):
        logger.info("Generating stake keys")
        cmd = (
            f"{self.cli} latest stake-address key-gen "
            "--verification-key-file /dev/stdout --signing-key-file /dev/stdout"
        )
        result = self.run_command.run(cmd)
        if result.stderr:
            logger.error(result.stderr)
            return None, None

        # Convert to a valid JSON array
        modified_response = "[" + result.stdout.replace("\r", "").replace("}\n{", "},\n{") + "]"
        parsed_data = json.loads(modified_response)

        signing_key = parsed_data[0]
        verification_key = parsed_data[1]

        logger.info(f"Stake signing key: {signing_key}")
        logger.info(f"Stake verification key: {verification_key}")
        return signing_key, verification_key

    def generate_cross_chain_keys(self):
        logger.info("Generating cross chain keys")
        pkey = ecdsa.SigningKey.generate(ecdsa.SECP256k1)

        pkey_hex = pkey.to_string().hex()
        vkey_bytes = pkey.get_verifying_key().to_string("compressed")
        vkey_hex = vkey_bytes.hex()

        blake2b = hashlib.blake2b(digest_size=28)
        blake2b.update(vkey_bytes)
        vkey_hash = blake2b.digest().hex()

        logger.info(f"Cross chain signing key: {pkey_hex}")
        logger.info(f"Cross chain verification key: {vkey_hex}")
        logger.info(f"Cross chain verification key hash: {vkey_hash}")

        return pkey, vkey_hex, vkey_hash

    def get_stake_key_hash(self, stake_key):
        logger.info("Getting stake key hash")
        cmd = f"{self.cli} latest stake-address key-hash --stake-verification-key {stake_key}"
        result = self.run_command.run(cmd)
        if result.stderr:
            logger.error(result.stderr)
        return result.stdout.strip()
