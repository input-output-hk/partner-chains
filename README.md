# Partner Chains Toolkit

:chains: Toolkit for maintaining and securing [Substrate](https://polkadot.com/) based blockchains with the Cardano ecosystem

![polkadot-sdk](https://img.shields.io/badge/polkadot--sdk-stable2506-blue)
[![language](https://img.shields.io/badge/language-Rust-239120)]()
[![OS](https://img.shields.io/badge/OS-linux%2C%20macOS-0078D4)]()
[![CPU](https://img.shields.io/badge/CPU-x64%2C%20ARM64-FF8C00)]()
[![GitHub release date](https://img.shields.io/github/release-date/input-output-hk/partner-chains)](#)
[![GitHub last commit](https://img.shields.io/github/last-commit/input-output-hk/partner-chains)](#)

_Note_: _This alpha release is just the beginning of the journey. It is intended to gather early feedback from the community and is provided "as is." It should not be used in live production networks. Use at your own risk. We welcome and appreciate your feedback!_


### Table of Contents
  * [About](#information_source-about)
  * [Prerequisites](#warning-prerequisites)
  * [How to Build](#hammer_and_wrench-how-to-build)
  * [How to Test](#white_check_mark-how-to-test)
  * [Documentation](#books-documentation)
  * [License](#-license)

### :information_source: About

The Partner Chains Toolkit provides a collection features and cli commands enabling users to
either **build new partner chains** (_=chain builder_), or **operate validators** provided
by the chain builder (_=validator_).

Relevant features include:

* **Minotaur**: A multi-resource selection algorithm combining validators from multiple networks
into a single joint-consensus mechanism. Version 1 is exclusive to Cardano.
* **D-Param**: Secure bootstrapping of a partner chain through a configurable ratio of public (registered)
versus permissioned validators.
* **Multi-sig Governance**: Decentralized and secure governance of a partner chain through
a set of Governance Authorities with a configurable threshold.
* **Native Token Reserve Management**: Initialization and maintenance of a secure token reserve on Cardano
that is observed on a partner chain.
* **Block Production Rewards**: Enabling the creation and maintenance of a reward system for block
producers and delegators.

### :warning: Prerequisites
While the toolkit is built with convenience and ease-of-use in mind, using it to build new partner chains requires
experience in several domains:
- Strong blockchain background in general
- Cardano ecosystem and tools
- Polkadot ecosystem and tools
- Rust programming language

### :hammer_and_wrench: How to Build
The project provides a [Nix](https://nixos.org/nix) environment that provides all necessary tools
and dependencies for building it (You can also choose to install dependencies manually but we won't
officially support or document this approach).
```bash
$ nix develop # enter the development shell
$ cargo build
```
**Note**: The first invocation of `nix develop` may take some time since it fetches all required
dependencies.

### :white_check_mark: How to Test
Please refer to our [Testing](./e2e-tests/README.md) documentation on how to run our end-to-end
tests. The unit tests can be invoked as expected using cargo:
```bash
$ cargo test
```
_Note_: _The tests make use of [testcontainers](https://rust.testcontainers.org/) so you will need
to have a working docker setup to execute the tests_.

### :books: Documentation
Please refer to our [Introduction](./docs/intro.md) to learn in more detail about what the Partner
Chain Toolkit offers and how to use it for different use-cases.

Rust Docs for all crates provided by the toolkit are available to browse online [here](https://input-output-hk.github.io/partner-chains/)

#### Correctness & Liveliness
In order to ensure and demonstrate the correctness and liveliness of our partner chain toolkit, we created
reproducible simulations of the D-Parameter based committee selection mechanism and created end-to-end tests
running hundreds of nodes to inspect and assert correct behavior:

- Refer to [docs/reports/ariadne-liveness.pdf](docs/reports/ariadne-liveness.pdf) for the simulation report.
- Refer to [docs/reports/liveliness-report.pdf](docs/reports/liveliness-report.pdf) for the end-to-end-testing report.

### 📃 License

This project is primarly distributed under the Apache License 2.0. You can review the full license
agreement at the following link: [LICENSE](./LICENSE).
Parts of the code are distributed under the GPL v3.0 with "Classpath exception". You can review
the full license at the following link:
[GPL v3.0 with Classpath exception](./LICENSE-GPL3-with-classpath-exception).
