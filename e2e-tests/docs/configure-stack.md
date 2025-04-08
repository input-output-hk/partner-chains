# How to configure remote host for testing

## What is stack?

Partner chains tests require a set of binaries for test execution:
- [partner-chains-node](https://github.com/input-output-hk/partner-chains) to generate signatures
- [cardano-cli](https://github.com/IntersectMBO/cardano-node?tab=readme-ov-file#using-cardano-cli) to query the tip of the Cardano main chain

As a user, you can choose where to place those services and binaries: on the test runner machine or in a Kubernetes pod.

## Set up stack on test runner machine

In case of the test runner machine (and local execution), you will need to update the binaries path in `stack_config.tools` for `<env>-stack.json` file in the `config/<blockchain>/<env>` folder:

- `cardano_cli`
- `partner_chains_node`

## Set up stack in Kubernetes

To configure the stack in Kubernetes, you will need to do the following:
1. Run cardano-node in a Kubernetes pod, expose the node socket file and make cardano-cli executable
2. Deploy the required binaries (cardano-cli, partner-chains-node, bech32) to the appropriate Kubernetes pods
3. Update `<env>-stack.json` file in the `config/<blockchain>/<env>` folder
   1. Set `stack_config.tools` to use kubectl exec to access the binaries in the pods
   2. Set `stack_config.validator_name` to the name of the validator pod to use

### `<env>_stack.json` template:

```
{
    "stack_config": {
        "ogmios_host": <STRING>,
        "ogmios_port": 1337,
        "validator_name": <STRING>,
        "tools": {
            "cardano_cli": {
                "cli": "cardano-cli",
                "shell": "kubectl exec -it ${stack_config[validator_name]} -c cardano-node -n <namespace> --"
            },
            "partner_chains_node": {
                "cli": "/usr/local/bin/partner-chains-node",
                "shell": "kubectl exec -it ${stack_config[validator_name]} -c substrate-node -n <namespace> --"
            },
            "bech32": {
                "cli": "bech32",
                "shell": "kubectl exec -it ${stack_config[validator_name]} -c substrate-node -n <namespace> --"
            }
        }
    }
}
