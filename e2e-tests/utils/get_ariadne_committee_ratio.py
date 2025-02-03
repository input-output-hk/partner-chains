import sys
import os
import json

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from src.partner_chain_rpc import PartnerChainRpc
from omegaconf import OmegaConf

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

devnet_keysdict = {
    "cc": {
        'Alice': '0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1',
        'Bob': '0x0390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f27',
        'Charlie': '0x0389411795514af1627765eceffcbd002719f031604fadd7d188e2dc585b4e1afb',
        'Dave': '0x03bc9d0ca094bd5b8b3225d7651eac5d18c1c04bf8ae8f8b263eebca4e1410ed0c',
        'Eve': '0x031d10105e323c4afce225208f71a6441ee327a65b9e646e772500c74d31f669aa',
        'Ferdie': '0x0291f1217d5a04cb83312ee3d88a6e6b33284e053e6ccfc3a90339a0299d12967c',
        'Greg': '0x02dacce90fca29ca80404d9b4e8ff3d9dabd03def6a82e412acb2ad04dd734dbfc',
        'Henry': '0x0263c9cdabbef76829fe5b35f0bbf3051bd1c41b80f58b5d07c271d0dd04de2a4e',
    },
    'aura': {
        'Alice': '0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d',
        'Bob': '0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48',
        'Charlie': '0x90b5ab205c6974c9ea841be688864633dc9ca8a357843eeacf2314649965fe22',
        'Dave': '0x306721211d5404bd9da88e0204360a1a9ab8b87c66c1bc2fcdd37f3c2222cc20',
        'Eve': '0xe659a7a1628cdd93febc04a4e0646ea20e9f5f0ce097d9a05290d4a9e054df4e',
        'Ferdie': '0x1cbd2d43530a44705ad088af313e18f80b53ef16b36177cd4b77b846f2a5f07c',
        'Greg': '0x2c4ed1038f6e4131c21b6b89885ed232c5b81bae09009376e9079cc8aa518a1c',
        'Henry': '0x9cedc9f7b926191f64d68ee77dd90c834f0e73c0f53855d77d3b0517041d5640',
    },
    'grandpa': {
        'Alice': '0x88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee',
        'Bob': '0xd17c2d7823ebf260fd138f2d7e27d114c0145d968b5ff5006125f2414fadae69',
        'Charlie': '0x439660b36c6c03afafca027b910b4fecf99801834c62a5e6006f27d978de234f',
        'Dave': '0x5e639b43e0052c47447dac87d6fd2b6ec50bdd4d0f614e4299c665249bbd09d9',
        'Eve': '0x1dfe3e22cc0d45c70779c1095f7489a8ef3cf52d62fbd8c2fa38c9f1723502b5',
        'Ferdie': '0x568cb4a574c6d178feb39c27dfc8b3f789e5f5423e19c71633c748b9acf086b5',
        'Greg': '0xfa41bacb202b0529288b05af1b324f85fe561091c2d29d9df1df37c3aa687c23',
        'Henry': '0xde21d8171821fc29a43a1ed90ee75623edc3794012010f165b6afc3483a569aa',
    },
    "ss58": {
        'Alice': '0x5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY',
        'Bob': '0x5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty',
        'Charlie': '0x5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y',
        'Dave': '0x5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy',
        'Eve': '0x5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw',
        'Ferdie': '0x5CiPPseXPECbkjWCa6MnjNokrgYjMqmKndv2rSnekmSK2DjL',
        'Greg': '0x5D4oNr3wasgzxt7KdoHBjnxUthfoM93kWMav63aoYmPHfkBW',
        'Henry': '0x5FcTxwLAQ8L23HvTa6Y6UUMBKJkYRG42Vg9wVVpZDpE2ZnTZ',
    },
}

staging_keysdict = {
    "cc": {
        "Validator-1": "0x03b827f4da9711bab7292e5695576a841a4d20af9a07b1ba7a230168d2a78e9df4",
        "Validator-2": "0x02ef5bcd94d54a18ad199559782cd72ac3ccd850976aaaafbca8f9d2625afbf7c4",
        "Validator-3": "0x02f2762ab6e1a125dc03908a7b738f8023d13763f28a11d7633c6c8bc463478430",
        "Validator-4": "0x025e19f82c5e2bac5e8869d49ff26359e442628bc5cfa38eeb5275f43d04015da8",
        "Validator-5": "0x03f38a062a4b372c045c1dddc4fe98a2c9cb1d6eec8bf02f973fd29b1096cd8155",
        "Validator-6": "0x033d3a2e581821fdd222581f6015eaabc798dd4dc0f7eeb3d6630b84449d76c9c9",
        "Validator-7": "0x0232ebed4c0c742fa951b471fe6f6f2f09a2d235bf7e9992fbf786cf032c97247e",
    }
}

TARGET_ENV = 'staging'
if TARGET_ENV == 'devnet':
    keysdict = devnet_keysdict
    NODE = 'http://10.0.10.13:30023'  # devnet charlie
elif TARGET_ENV == 'staging':
    keysdict = staging_keysdict
    NODE = 'http://10.0.11.16:9933'  # staging
elif TARGET_ENV == 'local':
    keysdict = devnet_keysdict
    NODE = 'http://localhost:9945'  # local alice


def main():
    partner_chain_epoch_calculator = PartnerChainEpochCalculator(config)
    partner_chain_rpc_instance = PartnerChainRpc(NODE)  # Create an instance of PartnerChainRpc
    current_status = partner_chain_rpc_instance.partner_chain_get_status().result
    current_mc_epoch = current_status['mainchain']['epoch']
    print(f"Current MC epoch: {current_mc_epoch}")
    current_pc_epoch = current_status['sidechain']['epoch']
    print(f"Current PC epoch: {current_pc_epoch}")
    mc_epoch = current_mc_epoch - 1  # Choose MC epoch to query
    print(f"MC epoch to query: {mc_epoch}")
    epoch_range = partner_chain_epoch_calculator.find_pc_epochs(mc_epoch)  # Get the PC epoch range of the MC epoch
    print(f"Current PC epoch range: {epoch_range}")
    d_param = partner_chain_rpc_instance.partner_chain_get_ariadne_parameters(mc_epoch).result
    start_pc_epoch = 4762485  # epoch_range.start
    end_pc_epoch = 4762485 + 5  # current_pc_epoch if mc_epoch == current_mc_epoch else epoch_range.stop
    print(f"PC epoch range to query: {start_pc_epoch} - {end_pc_epoch}")
    permissioned_counter = 0
    trustless_counter = 0

    permissioned_names = list(
        {
            name
            for name, key in keysdict['cc'].items()
            if key in [candidate['sidechainPublicKey'] for candidate in d_param['permissionedCandidates']]
        }
    )

    trustless_names = list(
        {
            name
            for name, key in keysdict['cc'].items()
            if key in [candidate[0]['sidechainPubKey'] for candidate in d_param['candidateRegistrations'].values()]
        }
    )

    f = open(f'Ariadne-committee-ratio-{TARGET_ENV}-{mc_epoch}.txt', 'w')
    f.write(f"Permissioned: {permissioned_names}\n")
    f.write(f"Trustless: {trustless_names}\n")
    f.write(f"numPermissionedCandidates: {d_param['dParameter']['numPermissionedCandidates']}\n")
    f.write(f"numRegisteredCandidates: {d_param['dParameter']['numRegisteredCandidates']}\n\n")

    for epoch in range(start_pc_epoch, end_pc_epoch):
        committee = partner_chain_rpc_instance.partner_chain_get_epoch_committee(epoch).result
        committee_pubKeys = committee['committee']
        inverted_dict = {v: k for k, v in keysdict['cc'].items()}
        committee_names = []
        for candidate in committee_pubKeys:
            committee_names.append(inverted_dict[candidate['sidechainPubKey']])

        trustless_count = sum([committee_names.count(name) for name in trustless_names])
        permissioned_count = sum([committee_names.count(name) for name in permissioned_names])
        trustless_counter += trustless_count
        permissioned_counter += permissioned_count

        print(committee_names)
        committee_names_len_bound = 3 + 10 * (
            d_param['dParameter']['numPermissionedCandidates'] + d_param['dParameter']['numRegisteredCandidates']
        )
        f.write(
            f"{epoch}: {json.dumps(committee_names):{committee_names_len_bound}}:"
            f" P/T: {permissioned_count}/{trustless_count}\n"
        )

    f.write(f"Average permissioned: {permissioned_counter / (end_pc_epoch - start_pc_epoch)}\n")
    f.write(f"Average trustless: {trustless_counter / (end_pc_epoch - start_pc_epoch)}\n")

    f.close()


if __name__ == '__main__':
    main()
