#!/bin/bash
set -e

# Input parameters
SHA=$1

# Repository and image configurations
IMAGE_REPO="689191102645.dkr.ecr.eu-central-1.amazonaws.com/substrate-node"

# Manifest configurations
MANIFEST_FILENAME="manifest-sha-$SHA.yaml"
MANIFEST_TEMPLATE="./manifest.yaml"

# GitHub configurations for the ArgoCD repository
ARGOCD_CONTENT_PATH="integration-testing/$MANIFEST_FILENAME"
ARGOCD_REPO_API_ENDPOINT="https://api.github.com/repos/input-output-hk/sidechains-argocd/contents/$ARGOCD_CONTENT_PATH"
BRANCH_NAME="main" 

# Message for the commit
MESSAGE="ci: Deploy integration-testing environment for SHA #$SHA"

# Create the new manifest file in the current directory
sed "s|{{SHA}}|$SHA|g; s|{{SUBSTRATE_NODE_IMAGE}}|$IMAGE_REPO:$SHA|g" "$MANIFEST_TEMPLATE" > "$MANIFEST_FILENAME"

# Encode file in Base64 as GitHub API expects this format
CONTENT=$(base64 -w 0 "$MANIFEST_FILENAME")

# Create or update the file in the ArgoCD repo using the GitHub API
gh api "$ARGOCD_REPO_API_ENDPOINT" \
  --method PUT \
  --field message="$MESSAGE" \
  --field content="$CONTENT" \
  --field branch="$BRANCH_NAME"