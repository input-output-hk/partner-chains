#!/usr/bin/env sh
docker build --pull -f dev/docker/builder.Dockerfile -t "partner-chains-node-builder:latest" --ssh default . && \
docker build -f dev/docker/Dockerfile -t partner-chains-node:latest docker
