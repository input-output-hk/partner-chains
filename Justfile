# Set the shell to bash with the desired flags
set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

shebang := '/usr/bin/env bash'

# Define variables at the top level without indentation
current_system := `nix eval --impure --expr 'builtins.currentSystem'`

# ------------------------------------------------------------------------------
# Default Target
# ------------------------------------------------------------------------------

# The `all` recipe serves as the default target.
[group('general')]
all: build

# ------------------------------------------------------------------------------
# Rust Targets
# ------------------------------------------------------------------------------
# Build the project
build:
  cargo build

# Check rustc and clippy warnings
[group('Rust')]
check:
  cargo check --all-targets
  cargo clippy --all-targets

# Automatically fix rustc and clippy warnings
[group('Rust')]
fix:
  cargo fix --all-targets --allow-dirty --allow-staged
  cargo clippy --all-targets --fix --allow-dirty --allow-staged

# ------------------------------------------------------------------------------
# Partner Chains Smart Contracts
# ------------------------------------------------------------------------------

# Run the npm package of smart contracts
[group('Partner Chains')]
contracts-cli:
  npx @partner-chains/pc-contracts-cli $@

# Switch devshells
[group('Partner Chains')]
switch shell:
  nix develop .#{{shell}}

# ------------------------------------------------------------------------------
# CI/CD
# ------------------------------------------------------------------------------

# Run earthly CI locally
[group('ci/cd')]
ci-build:
  earthly