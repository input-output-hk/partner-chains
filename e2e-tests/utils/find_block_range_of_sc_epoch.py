import sys
import os
import json
from omegaconf import OmegaConf

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from partner_chain_rpc import PartnerChainRpc
from config.api_config import ApiConfig
from src.substrate_api import SubstrateApi
from pc_block_finder import BlockFinder

STAGING_NODE = 'http://staging-preview-validator-1-service.staging-preview.svc.cluster.local:9933'  # staging
DEVNET_NODE = 'http://charlie-service.sc.svc.cluster.local:9933'  # devnet charlie
LOCAL_NODE = 'http://localhost:9945'  # local alice

STAGING_CONFIG = 'config/substrate/staging_nodes.json'
DEVNET_CONFIG = 'config/substrate/devnet_nodes.json'
LOCAL_CONFIG = 'config/substrate/local_nodes.json'

STAGING_STACK = 'config/substrate/staging_stack.json'
DEVNET_STACK = 'config/substrate/devnet_stack.json'
LOCAL_STACK = 'config/substrate/local_stack.json'

TARGET_ENV = 'staging'
if TARGET_ENV == 'staging':
    NODE = STAGING_NODE
    CONFIG = STAGING_CONFIG
    STACK = STAGING_STACK
elif TARGET_ENV == 'devnet':
    NODE = DEVNET_NODE
    CONFIG = DEVNET_CONFIG
    STACK = DEVNET_STACK
elif TARGET_ENV == 'local':
    NODE = LOCAL_NODE
    CONFIG = LOCAL_CONFIG
    STACK = LOCAL_STACK

with open('config/config.json', 'r') as f:
    config_json = json.load(f)

with open(CONFIG, 'r') as f:
    nodes_config_json = json.load(f)

with open(STACK, 'r') as f:
    stack_config_json = json.load(f)

default_config = OmegaConf.create(config_json)
nodes_config = OmegaConf.create(nodes_config_json)
stack_config = OmegaConf.create(stack_config_json)
schema = OmegaConf.structured(ApiConfig)
config: ApiConfig = OmegaConf.merge(schema, default_config, nodes_config, stack_config)


def main():

    partner_chain_rpc_instance = PartnerChainRpc(NODE)  # Create an instance of PartnerChainRpc
    current_status = partner_chain_rpc_instance.partner_chain_get_status().result
    api = SubstrateApi(config, None, None)
    blockCalculator = BlockFinder(api, config)

    # Replace this with the epoch you want to find the block range of
    target_pc_epoch = 4762499  # current_status['sidechain']['epoch'] - 1
    block_range = blockCalculator.get_block_range(
        current_status['sidechain']['nextEpochTimestamp'], current_status['sidechain']['epoch'], target_pc_epoch
    )

    print(f"Block range of epoch {target_pc_epoch}: {block_range}")


if __name__ == '__main__':
    main()
