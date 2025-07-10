#!/bin/bash

# Verify the e2e-tests directory exists and cd into it
if [ -d "/e2e-tests" ]; then
  cd /e2e-tests
else
  echo "Error: Directory /e2e-tests does not exist. Ensure e2e-tests directory was copied to ./configuration/tests/e2e-tests/ before bringing up the container"
  exit 1
fi

# 1) Purge any stale lists
apt-get clean
rm -rf /var/lib/apt/lists/*

# 2) Update with a few retries to work around mirror glitches
apt-get update -o Acquire::Retries=5

# 3) Install core bits, allowing missing/fixed-missing
apt-get install -y --fix-missing \
    docker.io curl

# 4) Download & install SOPS, then clean up again
curl -L https://github.com/getsops/sops/releases/download/v3.7.3/sops_3.7.3_amd64.deb -o /tmp/sops.deb
dpkg -i /tmp/sops.deb
rm /tmp/sops.deb
apt-get clean
rm -rf /var/lib/apt/lists/*

# 5) Create and initialize the virtual environment
python -m venv venv
source venv/bin/activate
pip install --upgrade pip
pip install -r requirements.txt

# Keep the container running
tail -f /dev/null
