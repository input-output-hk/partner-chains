#!/usr/bin/env sh
docker build --pull -f docker/builder.Dockerfile -t "sidechains-substrate-node-builder:latest" --ssh default . && \
# Heads up: there is name discrepancy. Locally, image repository is 'sidechains-substrate-node',
# but CI can only push image to 'substrate-node' repository into ECR.
docker build -f docker/Dockerfile -t sidechains-substrate-node:latest docker
