import os.path
import os
import sys
import pprint
import json

currentdir = os.path.dirname(os.path.realpath(__file__))
parentdir = os.path.dirname(currentdir)
sys.path.append(parentdir)
from partner_chain_rpc import PartnerChainRpc

PC_INITIAL_COMMITTEE = 2  # Number of pc epochs after mc epoch where new committe first appears
main_chain = {
    "network": "--testnet-magic 2",
    "epoch_length": 86400,
    "active_slots_coeff": 0.05,
    "security_param": 432,
    "init_timestamp": 1666656000,
}
partner_chain = {
    "block_duration": 6,
    "slots_in_epoch": 60,
}

devnet_rpc_url = "http://charlie-service.sc.svc.cluster.local:9933"  # devnet
staging_rpc_url = "http://staging-preview-validator-1-service.staging-preview.svc.cluster.local:9933"  # staging
halo2-qa_rpc_url = "http://node-01.halo2-qa.dev.platform.midnight.network:9944"  # halo2-qa

# first_epoch_to_get_committee_since_chain_wipe = 4706032
mc_epoch = 550

devnet_candidates = {
    "0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1": "Alice",
    "0x0390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f27": "Bob",
    "0x0389411795514af1627765eceffcbd002719f031604fadd7d188e2dc585b4e1afb": "Charlie",
    "0x03bc9d0ca094bd5b8b3225d7651eac5d18c1c04bf8ae8f8b263eebca4e1410ed0c": "Dave",
    "0x031d10105e323c4afce225208f71a6441ee327a65b9e646e772500c74d31f669aa": "Eve",
    "0x0291f1217d5a04cb83312ee3d88a6e6b33284e053e6ccfc3a90339a0299d12967c": "Ferdie",
    "0x02dacce90fca29ca80404d9b4e8ff3d9dabd03def6a82e412acb2ad04dd734dbfc": "Greg",
    "036c6ae73d36d0c02b54d7877a57b1734b8e096134bd2c1b829431aa38f18bcce1": "One",
}

staging_candidates = {
    "0x03b827f4da9711bab7292e5695576a841a4d20af9a07b1ba7a230168d2a78e9df4": "Validator-1",
    "0x02ef5bcd94d54a18ad199559782cd72ac3ccd850976aaaafbca8f9d2625afbf7c4": "Validator-2",
    "0x02f2762ab6e1a125dc03908a7b738f8023d13763f28a11d7633c6c8bc463478430": "Validator-3",
    "0x025e19f82c5e2bac5e8869d49ff26359e442628bc5cfa38eeb5275f43d04015da8": "Validator-4",
    "0x03f38a062a4b372c045c1dddc4fe98a2c9cb1d6eec8bf02f973fd29b1096cd8155": "Validator-5",
    "0x033d3a2e581821fdd222581f6015eaabc798dd4dc0f7eeb3d6630b84449d76c9c9": "Validator-6",
    "0x0232ebed4c0c742fa951b471fe6f6f2f09a2d235bf7e9992fbf786cf032c97247e": "Validator-7",
}

devnet_counter = {'Alice': 0, 'Bob': 0, 'Charlie': 0, 'Dave': 0, 'Eve': 0, 'Ferdie': 0, 'Greg': 0, 'One': 0}
staging_counter = {
    'Validator-1': 0,
    'Validator-2': 0,
    'Validator-3': 0,
    'Validator-4': 0,
    'Validator-5': 0,
    'Validator-6': 0,
    'Validator-7': 0,
}

TARGET_ENV = 'staging'
if TARGET_ENV == 'devnet':
    rpc_url = devnet_rpc_url
    counter = devnet_counter
    candidates = devnet_candidates
elif TARGET_ENV == 'staging':
    rpc_url = staging_rpc_url
    counter = staging_counter
    candidates = staging_candidates

partner_chain_rpc = PartnerChainRpc(rpc_url)


def get_first_pc_epoch(mc_epoch) -> int:
    mc_epoch_change_timestamp = mc_epoch * main_chain['epoch_length'] + main_chain['init_timestamp']
    pc_epoch = mc_epoch_change_timestamp / (partner_chain['block_duration'] * partner_chain['slots_in_epoch'])
    return int(pc_epoch)


def get_candidate_seats_num_from_ariadne_params(mc_epoch):
    params = partner_chain_rpc.partner_chain_get_ariadne_parameters(mc_epoch).result
    print(params)
    permissioned = len(params['permissionedCandidates'])
    trustless = len(params['candidateRegistrations'])
    print(permissioned, trustless)
    p_param = params['dParameter']['numPermissionedCandidates']
    t_param = params['dParameter']['numRegisteredCandidates']
    print(p_param, t_param)
    return p_param + t_param


with open(f"pastCommittees_{TARGET_ENV}_{mc_epoch}.json", "w") as fp:
    candidate_seats = get_candidate_seats_num_from_ariadne_params(mc_epoch)
    first_epoch_to_get_committee = get_first_pc_epoch(mc_epoch) + PC_INITIAL_COMMITTEE
    last_epoch_to_get_committee = get_first_pc_epoch(mc_epoch + 1) + PC_INITIAL_COMMITTEE - 1
    print(
        f"Getting {last_epoch_to_get_committee-first_epoch_to_get_committee+1} committees "
        f"from pc epoch {first_epoch_to_get_committee} to current {last_epoch_to_get_committee}. "
        f"These cover mc epoch {mc_epoch}."
    )
    committees = {}
    for epoch in range(last_epoch_to_get_committee, first_epoch_to_get_committee - 1, -1):
        committee = partner_chain_rpc.partner_chain_get_epoch_committee(epoch).result
        for candidate in committee['committee']:
            counter[candidates[candidate['sidechainPubKey']]] += 1
        committees[str(epoch)] = committee['committee']
    total_sum = 0
    for sum in counter.keys():
        total_sum += counter[sum]
    counter['_SUM'] = total_sum
    counter['_AVG'] = counter['_SUM'] / candidate_seats
    committees['Summary'] = counter
    committees['Summary']['mc_epoch'] = mc_epoch
    pprint.pprint(committees)
    json.dump(committees, fp, indent=4, sort_keys=True)
    fp.write(",\n")


# def main():
#     for epoch in range(4761600, 4761840):
#         out_txs = partner_chain_rpc.partner_chain_get_outgoing_transactions(epoch)['transactions']
#         if out_txs != []:
#             print(f"Epoch {epoch}:")
#             print(out_txs)


# if __name__ == '__main__':
#     main()
