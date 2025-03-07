from enum import Enum
from .substrate_api import SubstrateApi


class BlockchainTypes(Enum):
    substrate = SubstrateApi
    midnight = SubstrateApi
