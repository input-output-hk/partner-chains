#!/bin/bash

# Verify the E2E-tests directory exists and cd into it
if [ -d "/E2E-tests" ]; then
  cd /E2E-tests
else
  echo "Error: Directory /E2E-tests does not exist. Ensure E2E-tests directory was copied to ./configuration/tests/E2E-tests/ before bringing up the container"
  exit 1
fi

# Install Docker CLI for running Docker commands in other containers
apt-get update && apt-get install -y docker.io

# Install pytest dependencies
apt-get update && \
apt-get install -y curl && \
curl -L --silent https://github.com/getsops/sops/releases/download/v3.7.3/sops_3.7.3_amd64.deb > sops.deb && \
dpkg -i sops.deb && \
rm sops.deb && \
apt-get clean && \
rm -rf /var/lib/apt/lists/*

# Create and initialize the virtual environment
python -m venv venv
source venv/bin/activate
pip install --upgrade pip
pip install -r requirements.txt

# Keep the container running
tail -f /dev/null
