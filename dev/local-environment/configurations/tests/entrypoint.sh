#!/bin/bash

if [ -d "/e2e-tests" ]; then
  cd /e2e-tests
else
  echo "Error: Directory /e2e-tests does not exist. Ensure e2e-tests directory was copied to ./configuration/tests/e2e-tests/ before bringing up the container"
  exit 1
fi

apt-get update && \
apt-get install -y curl && \
curl -L --silent https://github.com/getsops/sops/releases/download/v3.7.3/sops_3.7.3_amd64.deb > sops.deb && \
dpkg -i sops.deb && \
rm sops.deb && \
apt-get clean && \
rm -rf /var/lib/apt/lists/*

python -m venv venv
source venv/bin/activate

pip install --upgrade pip
pip install -r requirements.txt

exec "$@"
