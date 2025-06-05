# Stack configuration

## What is stack?

The stack consists of two elements:
- ogmios
- tools

### Example stack configuration

```
{
    "stack_config": {
        "ogmios_scheme": "ws",
        "ogmios_host": "staging-preview-services-service.staging-preview.svc.cluster.local",
        "ogmios_port": 1337,
        "tools": {
            ...
        }
    }
}
```

## Ogmios

Ogmios service is needed for the smart contracts to work. Make sure that it's accessible in any location you run your tests from.

## Tools

Partner chains tests require a set of binaries for test execution:
- [node](https://github.com/input-output-hk/partner-chains) to interact with smart contracts, generate signatures etc.
- [cardano-cli](https://github.com/IntersectMBO/cardano-node?tab=readme-ov-file#using-cardano-cli) to query Cardano main chain state

As a user, you need to configure paths to these executables. Currently `Runner` and `RunnerConfig` classes support kubernetes and docker executors. If needed this may be extended to local or SSH based solution.

### Example docker configuration

```
"tools": {
    "cardano_cli": {
        "path": "cardano-cli",
        "runner": {
            "docker": {
                "container": "cardano-node-1"
            }
        }
    },
    "node": {
        "path": "./partner-chains-node",
        "runner": {
            "docker": {
                "container": "partner-chains-setup"
            }
        }
    }
}
```

In this example, each tool will execute commands via `docker exec -c <container> bash -c "<command>"`.

### Example kubernetes configuration

```
"tools": {
    "runner": {
        "kubernetes": {
            "pod": "staging-preview-validator-1",
            "namespace": "staging-preview"
        }
    },
    "cardano_cli": {
        "path": "cardano-cli",
        "runner": {
            "kubernetes": {
                "container": "cardano-node"
            }
        }
    },
    "node": {
        "path": "/usr/local/bin/partner-chains-node",
        "runner": {
            "workdir": "/data/e2e-tests",
            "copy_secrets": true,
            "kubernetes": {
                "container": "substrate-node"
            }
        }
    }
}
```

In this example, you can observe default runner configuration for both tools: kubernetes pod and namespace.
Each of the tools has its own container specified, since the tools exists in different containers.
You can also override default values for any tool.

The commands will be executed via `kubectl exec <pod> -c <container> -n <namespace> -- bash -c "<command>"`.

This example also reveals two additional config options that you can set: `workdir` and `copy_secrets`.

### Configuring working directory

Runner commands are executed in the same directory that you get into when entering the shell.
If necessary, user can configure working directory by specifying `workdir`:
```
"runner": {
    "workdir": "/data/e2e-tests",
    ...
}
```

Working directory also impacts the location of any temp file that is created during test execution.

For example, `test_upsert_permissioned_candidates` uses `write_file` fixture to save file with public keys on the same container that `node` is.
If `workdir` is specified, the temp file will be created under `<workdir>/tmp.XXXXXXXXXX`.
If `workdir` is not set, any temp files are saved to `/tmp/tmp.XXXXXXXXXX`.

### Copying secrets

Some tests require signing keys to be accessible by the `node` tool so that given operation might be completed, e.g. `node smart-contracts register -k <payment key file>`.
If those secrets are not stored alongside tools, user can explicitly configure the stack to copy them into a temporary file.
```
"runner": {
    "copy_secrets": true,
    ...
}
```

Temp file are deleted after the test completes.
