# How to configure remote host for testing

## What is stack?

Partner chains tests require a set of binaries for test execution:
- [partner-chains-node](https://github.com/input-output-hk/partner-chains) to generate signatures
- [cardano-cli](https://github.com/IntersectMBO/cardano-node?tab=readme-ov-file#using-cardano-cli) to query the tip of the Cardano main chain

As a user, you can choose where to place those services and binaries: on the test runner machine or a separate remote host.

## Set up stack on test runner machine

In case of the test runner machine (and local execution), you will need to update the binaries path in `stack_config.tools` for `<env>-stack.json` file in the `config/<blockchain>/<env>` folder:

- `cardano_cli`
- `partner_chains_node`

## Set up stack on remote host

To configure the stack, you will need to do the following:
1. Run cardano-node, expose the node socket file and make cardano-cli executable on remote host
2. Create an SSH key for the remote host
3. Add the SSH public key to the `config/<blockchain>` folder
4. Add the SSH key to the `secrets/<blockchain>/<env>/keys` folder
5. Update `<env>-stack.json` file in the `config/<blockchain>/<env>` folder
   1. Set `stack_config.tools_host` to the IP address of remote host
   2. Set `stack_config.ssh.username` and `stack_config.ssh.port`
   3. Set `stack_config.ssh.host_keys_path` to the file added at step 3
   4. Set `stack_config.ssh.private_key_path` to the file added at step 4
6. Set `tools.cardano_cli.cli` to the path to the cardano-cli binary. Do not forget about exposing CARDANO_NODE_SOCKET_PATH. E.g. `export CARDANO_NODE_SOCKET_PATH=/ipc/node.socket && /cardano-cli`
7. Set `tools.partner_chains_node` to the path to the `partner-chains-node` binary

### `<env>-stack.json` template:

```
{
    "stack_config": {
        ...
        "tools_host": <STRING>,
        "ssh": {
            "username": <STRING>,
            "host": "${stack_config[tools_host]}",
            "port": 22,
            "host_keys_path": "config/<blockchain>/known_hosts",
            "private_key_path": "secrets/<blockchain>/<env>/keys/ssh-key.yaml.decrypted"
        },
        "tools": {
            "cardano_cli": {
                "cli": <STRING>,
                "ssh": "${stack_config[ssh]}"
            },
            "partner_chains_node": {
                "cli": <STRING>,
                "ssh": "${stack_config[ssh]}"
            },
            "bech32": {
                "cli": <STRING>,
                "ssh": "${stack_config[ssh]}"
            }
        }
    }
}
