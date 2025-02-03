from substrateinterface import SubstrateInterface
import json

alice = 'ws://10.0.11.120:30021'
bob = 'ws://10.0.11.120:30022'
charlie = 'ws://10.0.11.120:30023'
dave = 'ws://10.0.11.120:30024'
eve = 'ws://10.0.11.120:30025'
ferdie = 'ws://10.0.11.120:30026'


def get_latest_pc_block_number(node_api, custom_type_registry):
    block = node_api.get_block()
    return block["header"]


def namestr(obj, namespace):
    return [name for name in namespace if namespace[name] is obj]


def main():
    with open("src/runtime_api.json") as file:
        custom_type_registry = json.load(file)

    nodes = [alice, bob, charlie, dave, eve, ferdie]

    nodes_dict = {}
    for node in nodes:
        substrate = SubstrateInterface(url=node, type_registry=custom_type_registry)
        nodes_dict[namestr(node, globals())[0]] = substrate

    for node in nodes_dict:
        block_header = get_latest_pc_block_number(nodes_dict[node], custom_type_registry)
        print(f"{node}\t -\t{block_header['number']} - \t{block_header['hash']}")


if __name__ == '__main__':
    main()
