from omegaconf import OmegaConf
import json
from config.api_config import ApiConfig
from src.pc_epoch_calculator import PartnerChainEpochCalculator

with open('config/config.json', 'r') as f:
    config_json = json.load(f)

with open('config/substrate/devnet_nodes.json', 'r') as f:
    nodes_config_json = json.load(f)

with open('config/substrate/local_stack.json', 'r') as f:
    stack_config_json = json.load(f)

default_config = OmegaConf.create(config_json)
nodes_config = OmegaConf.create(nodes_config_json)
stack_config = OmegaConf.create(stack_config_json)
schema = OmegaConf.structured(ApiConfig)
config: ApiConfig = OmegaConf.merge(schema, default_config, nodes_config, stack_config)

calc = PartnerChainEpochCalculator(config)


def mc_2_pc(mc_epoch: int):
    epochs_range = calc.find_pc_epochs(mc_epoch)
    print(
        f"PC epochs range is {calc.range_in_math_notation(epochs_range)}, both included."
        f"It's {len(epochs_range)} epochs."
    )
    return epochs_range


def pc_2_mc(pc_epoch: int, current_mc_epoch: int):
    mc_epoch = calc.find_mc_epoch(pc_epoch, current_mc_epoch)
    if mc_epoch:
        mc_epochs_range = calc.range_in_math_notation(calc.find_pc_epochs(mc_epoch))
        print(f"PC epoch {pc_epoch} found in MC epoch {mc_epoch} == range{mc_epochs_range}")
        return mc_epoch
    else:
        raise Exception(f"PC epoch {pc_epoch} not found in MC epochs range[1-{current_mc_epoch}]")


def main():

    print("Welcome to PC epoch calculator!")
    mode = int(input("Which mode you want to enter? Type: (1) MC->PC, or (2) PC->MC\n"))
    if mode == 1:
        mc_epoch = int(input("Enter MC epoch: "))
        mc_2_pc(mc_epoch, config)
    elif mode == 2:
        pc_epoch = int(input("Enter PC epoch: "))
        current_mc_epoch = int(input("Enter current MC epoch: "))
        pc_2_mc(pc_epoch, current_mc_epoch)
    else:
        raise Exception("Unknown calculator mode. Type: (1) MC->PC, or (2) PC->MC")


if __name__ == '__main__':
    main()
