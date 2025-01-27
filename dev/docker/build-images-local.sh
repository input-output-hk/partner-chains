#!/usr/bin/env sh
# This script has to be executed from the root of the repository to enable proper context for the Docker build. `./dev/docker/build-images-local.sh`
docker build --pull -f dev/docker/builder.Dockerfile -t "partner-chains-node-builder:latest" --ssh default . && \
docker build -f dev/docker/Dockerfile -t partner-chains-node:latest dev/docker
