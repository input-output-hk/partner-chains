# How to configure a remote host for testing

## Prerequisites

Partner chains tests require the following binaries to be available on the test runner machine:

- cardano-cli
- partner-chains-node
- bech32

## Setup

### Test Runner Machine

1. Set up the stack on your test runner machine
2. Set stack_config.validator_name to the name of the validator pod to use
3. Set stack_config.namespace to the Kubernetes namespace where your pods are running
4. Set stack_config.kubectl.pod to the name of the pod to use (defaults to validator_name)
5. Set stack_config.kubectl.namespace to the namespace where your pods are running (defaults to namespace)
6. Set stack_config.kubectl.container to the container name if different from the default

### Kubernetes Setup

1. Set up the stack in your Kubernetes cluster
2. Set stack_config.validator_name to the name of the validator pod to use
3. Set stack_config.namespace to the Kubernetes namespace where your pods are running
4. Set stack_config.kubectl.pod to the name of the pod to use (defaults to validator_name)
5. Set stack_config.kubectl.namespace to the namespace where your pods are running (defaults to namespace)
6. Set stack_config.kubectl.container to the container name if different from the default

## Secret Key Handling

When running tests that require secret keys (such as committee tests), the system will automatically:

1. Create a temporary directory in the validator pod
2. Copy the required secret keys to this temporary directory using `kubectl cp`
3. Update the configuration to use these temporary paths
4. Clean up the temporary directory after the test completes

This approach ensures that secret keys are securely handled within the Kubernetes environment.

## Template

Here's a template for the `<env>_stack.json` file:

```json
{
    "stack_config": {
        "ogmios_host": "<ogmios_host>",
        "ogmios_port": 1337,
        "validator_name": "<validator_pod_name>",
        "namespace": "<namespace>",
        "kubectl": {
            "pod": "${stack_config[validator_name]}",
            "namespace": "${stack_config[namespace]}",
            "container": "<container_name>"  // optional
        },
        "tools": {
            "cardano_cli": {
                "cli": "cardano-cli",
                "shell": ""
            },
            "partner_chains_node": {
                "cli": "/usr/local/bin/partner-chains-node",
                "shell": ""
            },
            "bech32": {
                "cli": "/tools/bech32",
                "shell": ""
            }
        }
    }
}
```

Replace `<ogmios_host>`, `<validator_pod_name>`, `<namespace>`, and optionally `<container_name>` with your actual values.

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
   1. Set `stack_config.kubectl` to configure the pod and namespace
   2. Set `stack_config.validator_name` to the name of the validator pod to use
   3. Set `stack_config.namespace` to the namespace where your pods are running

### `<env>_stack.json` template:

```
{
    "stack_config": {
        "ogmios_host": <STRING>,
        "ogmios_port": 1337,
        "validator_name": <STRING>,
        "namespace": <STRING>,
        "kubectl": {
            "pod": "${stack_config[validator_name]}",
            "namespace": "${stack_config[namespace]}",
            "container": "<container_name>"  // optional
        },
        "tools": {
            "cardano_cli": {
                "cli": "cardano-cli",
                "shell": ""
            },
            "partner_chains_node": {
                "cli": "/usr/local/bin/partner-chains-node",
                "shell": ""
            },
            "bech32": {
                "cli": "/tools/bech32",
                "shell": ""
            }
        }
    }
}
