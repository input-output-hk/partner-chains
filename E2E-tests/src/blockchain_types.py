from enum import Enum
from .substrate_api import SubstrateApi
from .pc_evm_api import PartnerChainEvmApi


class BlockchainTypes(Enum):
    substrate = SubstrateApi
    midnight = SubstrateApi
    pc_evm = PartnerChainEvmApi
