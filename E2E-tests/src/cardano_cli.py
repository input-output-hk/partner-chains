import json
from .run_command import RunnerFactory
from config.api_config import MainChainConfig, Tool
import logging as logger


class CardanoCli:
    def __init__(self, config: MainChainConfig, cardano_cli: Tool):
        self.cli = cardano_cli.cli
        self.network = config.network
        self.run_command = RunnerFactory.get_runner(cardano_cli.ssh, cardano_cli.shell)

    def query_tip(self, node_num=None) -> int:
        socket_path = "/data/node.socket"

        cmd = f"export CARDANO_NODE_SOCKET_PATH={socket_path} && /tools/cardano-cli query tip {self.network}"

        result = self.run_command.run(cmd)
        if result.stderr:
            logger.error(f"Error querying tip: {result.stderr}")
        if not result.stdout:
            logger.error("Empty response from query tip command")
            raise ValueError(f"No output from command: {cmd}")

        try:
            return json.loads(result.stdout)
        except json.JSONDecodeError as e:
            logger.error(f"Failed to parse JSON response: '{result.stdout}'")
            logger.error(f"Command used: {cmd}")
            raise

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
        logger.info('Getting list of tokens and ADA with amounts...')
        utxosJson = self.get_utxos(address)
        tokensDict = {}
        for utxo in utxosJson.keys():
            for token_policy in utxosJson[utxo]['value'].keys():
                if token_policy == 'lovelace':
                    if 'ADA' in tokensDict.keys():
                        tokensDict['ADA'] += utxosJson[utxo]['value'][token_policy]
                    else:
                        tokensDict['ADA'] = utxosJson[utxo]['value'][token_policy]
                else:
                    for token_name in utxosJson[utxo]['value'][token_policy].keys():
                        if token_policy + '.' + token_name in tokensDict.keys():
                            tokensDict[token_policy + '.' + token_name] += utxosJson[utxo]['value'][token_policy][
                                token_name
                            ]
                        else:
                            tokensDict[token_policy + '.' + token_name] = utxosJson[utxo]['value'][token_policy][
                                token_name
                            ]
        return tokensDict

    def get_stake_pool_id(self, cold_vkey_file, cold_vkey=None):
        logger.info("Getting Stake Pool Id")
        if cold_vkey:
            cmd = f'{self.cli} latest stake-pool id --stake-pool-verification-key {cold_vkey} --output-format "hex"'
        else:
            cmd = f'{self.cli} latest stake-pool id --cold-verification-key-file {cold_vkey_file} --output-format "hex"'
        result = self.run_command.run(cmd)
        if result.stderr:
            logger.error(result.stderr)
        pool_id = result.stdout.strip()
        logger.info(f"Stake Pool Id: {pool_id}")
        return pool_id

    def get_stake_snapshot_of_pool(self, pool_id):
        logger.info("Getting pool's stake distribution")
        cmd = f'{self.cli} latest query stake-snapshot {self.network} --stake-pool-id {pool_id}'
        result = self.run_command.run(cmd)
        if result.stderr:
            logger.error(result.stderr)
        return json.loads(result.stdout)