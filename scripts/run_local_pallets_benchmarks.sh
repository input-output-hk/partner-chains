#!/usr/bin/env bash

# This script automates benchmarking for specific pallets in a Substrate node and builds the node image.
# Inspired by: https://github.com/paritytech/polkadot-sdk/blob/master/substrate/scripts/run_all_benchmarks.sh
# It is part of a workflow that runs this script inside a Docker container to mimic Kubernetes runtime environment, which can be found here:
# .github/workflows/generate_weights.yml

# The workflow matches Kubernetes allocated resources, includes Rust setup and script execution.
# Generated weights and errors are copied out of the container and uploaded for review.

# Once the workflow has been run, artifacts can be downloaded as a zip and the below weights existing in the repo can be updated / overwritten:
# /build/pallets/session-validator-management/src/weights.rs

set -e
# Build the Substrate node for production with runtime-benchmarks feature enabled.
cargo build --profile=production --features runtime-benchmarks
set +e

# Specify the path to the node executable to use for benchmarking.
SIDECHAIN=./target/production/partner-chains-node

# Define pallets to benchmark. Add or remove pallets as needed.
PALLETS=(
    "pallet_session_validator_management"
)

# Prepare an error file to log benchmarking issues. Cleans up before each run.
ERR_FILE="./scripts/benchmarking_errors.txt"
rm -f $ERR_FILE # Delete the existing error file to start fresh.

# Loop through each pallet and run benchmarks.
for PALLET in ${PALLETS[@]}
do
  # Determine the folder and weight file based on pallet name.
  FOLDER="$(echo "${PALLET#*_}" | tr '_' '-')"
  WEIGHT_FILE="./pallets/${FOLDER}/src/weights.rs"
  echo "[+] Benchmarking $PALLET with weight file $WEIGHT_FILE"

  # Run the benchmark with defined steps and repeats. Adjust these parameters as necessary.
  # Captures the output, including any errors, to the error file.
  OUTPUT=$(
    $SIDECHAIN benchmark pallet \
    --steps=50 \
    --repeat=20 \
    --pallet="$PALLET" \
    --extrinsic="*" \
    --wasm-execution=compiled \
    --heap-pages=4096 \
    --output="$WEIGHT_FILE" \
    --template=./.maintain/frame-weight-template.hbs 2>&1
  )
  if [ $? -ne 0 ]; then
    echo "$OUTPUT"
    echo "$OUTPUT" >> "$ERR_FILE"
    echo "[-] Failed to benchmark $PALLET. Error written to $ERR_FILE; continuing..."
  fi
done
