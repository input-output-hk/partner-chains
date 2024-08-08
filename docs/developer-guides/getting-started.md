# Getting started

This is a guide for partner chains developers to get started in the project. When you complete it, you should be able to build the project, test locally, and access development environment hosted in EKS.

Prerequisites:
* `nix` installed
* `docker` installed
* `protoc` installed [link](https://google.github.io/proto-lens/installing-protoc.html)

## Nix

The development shells use Nix with flakes enabled. It should provide the most
important tools (besides `docker`), to work in this project: `cargo` for
building the project, and `aws` and `kubectl` to interact with the development
environment.

[Install a recent version of Nix](https://zero-to-nix.com/start/install), and
use it to enter the project shell, preferably with [`direnv`](#direnv-support),
but `nix develop` will also work.

In order for nix to access our private repositories, it needs to be aware of a
github token. Create a `.netrc` file in your home directory with the following
content:
```
machine github.com
login <GITHUB_LOGIN>
password <GITHUB_TOKEN>

machine api.github.com
login <GITHUB_LOGIN>
password <GITHUB_TOKEN>
```

`GITHUB_TOKEN` should have following permissions:
- repo (full)
- read:org

It is recommended to add at least the following snippet to your
`/etc/nix/nix.conf` for all methods:
```
extra-experimental-features = nix-command flakes
netrc-file = /home/<USERNAME>/.netrc
```


After the token is setup, move to this project's directory and allow direnv to
read the `.envrc` file:
```
# if you need to enable direnv on the project for the first time
direnv allow

# or reload it if the nix content has changed
direnv reload
```

Build the project, to verify Rust toolchain is available:
```
cargo build
```


## Direnv support

For those using [direnv](https://direnv.net/), you may choose to use it to enter
the Nix shell since it not only allows you to use your shell of choice, but it
will also load the environmental variables used in development of this project.

It is highly recommended that you use this route not only for the
above-mentioned benefits, but because it will allow your shell to survive a
garbage collection, making entering it super quick after the initial build.

## AWS Account

To work with the Kubernetes cluster hosting developers environment,
a user created in AWS Account 689191102645 is required.
Terraform files controlling the environment are placed in the **sidechains-infra-priv**
repository. To get a user, create a PR to the **master** branch, it should contain
the entry with GPG public key and group assignment.
See https://github.com/input-output-hk/sidechains-infra-priv/pull/36 for reference.
When your PR is merged and terraform applied, SREs should give you the
first password for your user.
Log in https://eu-central-1.console.aws.amazon.com/console/home?region=eu-central-1#
and change it.

## AWS and EKS

Setup AWS and EKS configuration files.

1. Log in to AWS web console, from "user menu" go to "Security credentials",
   and create Access Keys, then setup ~/.aws/credentials file using **Long-term credentials**
   tab of this guide: https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-files.html.
2. kubectl config: https://docs.aws.amazon.com/eks/latest/userguide/create-kubeconfig.html
```
aws eks update-kubeconfig --region eu-central-1 --name iog-sidechain-substrate-kubernetes
# test
kubectl get pods -n sc
```
3. To log in into ECR, to pull or push images hosted there, run following command:
```
aws ecr get-login-password --region eu-central-1 | docker login --username AWS --password-stdin 689191102645.dkr.ecr.eu-central-1.amazonaws.com
```

## Testing locally

To be able to test your changes locally, you need some number of nodes running locally, depending on how much functionality is tested. One node for a smoke test, checking if application reads configuration properly. Two nodes are required for block production.

Command below show how to run a node locally:
```
./target/debug/sidechains-substrate-node --alice --base-path .run/data/alice --chain local --validator --node-key 0000000000000000000000000000000000000000000000000000000000000001  --port 30033 --rpc-port 3333 --unsafe-rpc-external --rpc-cors=all --state-pruning archive --blocks-pruning archive
```
`--alice` is one of special flags, that adhere to keys we usually use on devnet.
Others are bob, charlie, dave, eve, ferdie and greg.
`--chain local` makes node use the Local Testnet chain specification (chain_spec.rs file).
`--node-key` please use from 1 to N with 0s prefix.
`--state-pruning archive --blocks-pruning archive` are important, to test all endpoints we need
archive nodes.

Nodes read configuration from environment. `.envrc` file in this repository should be in-sync
with partner chain used by devnet, which usually is an initialized and running chain. Devnet partner chains can sometimes be used to test committee-rotation. For more extensive testing one should [create their own partner chain on the main chain](./user-guides/chain-builder.md). Then [run the required dependencies](dependencies.md) and test the system.

Please update partner-chains-smart-contracts revision in flake.nix, if your changes need a different version of partner-chains-smart-contracts than currently committed.
