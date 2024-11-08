#!/usr/bin/env sh
docker build --pull -f docker/builder.Dockerfile -t "partner-chains-node-builder:latest" --ssh default . && \
docker build -f docker/Dockerfile -t partner-chains-node:latest docker
