# Building a relay image compatible with sidechains devnet

The final effect of this procedure is a relay image for devnet pushed into the devnet ECR.

1. Checkout [input-output-hk/sidechains-substrate-relay](https://github.com/input-output-hk/sidechains-substrate-relay) repo
   and build a [Docker image](https://github.com/input-output-hk/sidechains-substrate-relay#building).
2. On partner-chains repo, build trustless-sidechain CLI image:
   ```
   trustless-sidechain-cli-image:load:docker
   ...
   Copy to Docker daemon image sidechain-main-cli-docker:fbea70
   ...
   ```
3. `export RELAY_IMAGE_TAG=<version, like 2.7.0>`
4. `export TRUSTLESS_SIDECHAIN_IMAGE_TAG=<short rev, should match output from docker build>`
5. `export TESTNET=<devnet or staging>`
6. Run build-docker-image.sh from partner-chains/docker/relay
