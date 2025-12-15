#!/bin/bash
set -ex  # Exit on any error, print commands

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

# 5) Install uv and sync dependencies
curl -LsSf https://astral.sh/uv/install.sh | sh
export PATH="/root/.local/bin:$PATH"
echo "uv version: $(uv --version)"
echo "Creating virtual environment with uv sync..."
uv sync --verbose
echo "Virtual environment created at: $(pwd)/.venv"
ls -la .venv/bin/

set +x  # Stop printing commands
echo "===== Environment setup complete ====="

# Keep the container running
tail -f /dev/null
