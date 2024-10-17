from substrateinterface import SubstrateInterface, Keypair, KeypairType
import json
from scalecodec import ss58_encode


def get_pc_balance(substrate, address):
    balance = substrate.query("System", "Account", [address])["data"]["free"]
    return balance.value


# NODE = 'ws://10.0.10.13:30023'  # charlie devnet
NODE = 'ws://10.0.10.55:9933'  # validator 1 staging


def _keypair_name_to_type(type_name):
    match type_name:
        case 'SR25519':
            return KeypairType.SR25519
        case 'ED25519':
            return KeypairType.ED25519
        case _:
            return KeypairType.ECDSA


class Wallet:
    raw = None
    address: str
    private_key: str
    mnemonic: str
    crypto_type: int
    seed: str
    public_key: str


def get_wallet(address=None, public_key=None, secret=None, scheme=None):
    scheme_type = _keypair_name_to_type(scheme)
    keypair = Keypair(crypto_type=scheme_type, ss58_format=42, private_key=secret, seed_hex=bytes.fromhex(secret))

    # keypair.ss58_address = address
    keypair.public_key = bytes.fromhex(public_key)
    wallet = Wallet()
    wallet.raw = keypair
    wallet.address = keypair.ss58_address
    print('wallet address', wallet.address)
    print('pub key:', keypair.public_key)
    wallet.private_key = keypair.private_key
    wallet.crypto_type = keypair.crypto_type
    wallet.public_key = keypair.public_key
    wallet.seed = keypair.seed_hex
    print("seed", keypair.seed_hex)
    print("mnemonic", keypair.mnemonic)
    print("ss58", keypair.create_from_private_key(keypair.private_key))
    ss58_address = ss58_encode(keypair.public_key, 42)
    print(ss58_address)
    return wallet


class MyClass:
    """A simple example class"""

    name = 'ferdie'

    def f(self):
        return self.name


def main():
    with open("src/runtime_api.json") as file:
        custom_type_registry = json.load(file)
    substrate = SubstrateInterface(url=NODE, type_registry=custom_type_registry)
    # balance = get_pc_balance(substrate, '5D4UdLBfePonjFvS1brdNA7sh2zpPvTh6PSEVihx6kPciPXD')  # passive
    # balance = get_pc_balance(substrate, '5HgahNbxK7syB2M2VB9iUJ6dcuHidCuP2jx2pJVidxRrKSng')  # active
    # balance = get_pc_balance(substrate, '5F1N52dZx48UpXNLtcCzSMHZEroqQDuYKfidg46Tp37SjPcE')  # negative-tes
    # # balance = get_pc_balance(substrate, '5D4oNr3wasgzxt7KdoHBjnxUthfoM93kWMav63aoYmPHfkBW')  # greg
    balance = get_pc_balance(substrate, '5C7C2Z5sWbytvHpuLTvzKunnnRwQxft1jiqrLD5rhucQ5S9X')  # staging faucet 0
    # balance = get_pc_balance(substrate, '5F1N52dZx48UpXNLtcCzSMHZEroqQDuYKfidg46Tp37SjPcE')  # staging funded QA account

    print(balance)


if __name__ == '__main__':
    main()
