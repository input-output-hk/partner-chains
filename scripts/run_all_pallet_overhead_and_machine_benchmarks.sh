#!/usr/bin/env bash

# This file was copied from substrate/frame/session/src/lib.rs and modified to fit this repo structure.
# Modifications can be seen by comparing the two files.

# This file is part of Substrate.
# Copyright (C) Parity Technologies (UK) Ltd.
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
# http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

# This script has three parts which all use the Substrate runtime:
# - Pallet benchmarking to update the pallet weights
# - Overhead benchmarking for the Extrinsic and Block weights
# - Machine benchmarking
#
# Should be run on a reference machine to gain accurate benchmarks

export USE_MOCK_DATA_SOURCES=true

while getopts 'bfp:v' flag; do
  case "${flag}" in
    b)
      skip_build='true'
      ;;
    f)
      # Fail if any sub-command in a pipe fails, not just the last one.
      set -o pipefail
      # Fail on undeclared variables.
      set -u
      # Fail if any sub-command fails.
      set -e
      # Fail on traps.
      set -E
      ;;
    p)
      # Start at pallet
      start_pallet="${OPTARG}"
      ;;
    v)
      # Echo all executed commands.
      set -x
      ;;
    *)
      echo "Bad options. Check Script."
      exit 1
      ;;
  esac
done


if [ "$skip_build" != true ]
then
  echo "[+] Compiling Substrate benchmarks..."
  cargo build --profile=production --locked --features=runtime-benchmarks --bin partner-chains-node
fi

SUBSTRATE=./target/production/partner-chains-node

# Manually exclude some pallets.
EXCLUDED_PALLETS=(
  "frame_benchmarking"
)

# Load all pallet names in an array.
ALL_PALLETS=($(
  $SUBSTRATE benchmark pallet --list=pallets --no-csv-header
))

# Filter out the excluded pallets by concatenating the arrays and discarding duplicates.
PALLETS=($({ printf '%s\n' "${ALL_PALLETS[@]}" "${EXCLUDED_PALLETS[@]}"; } | sort | uniq -u))

echo "[+] Benchmarking ${#PALLETS[@]} Substrate pallets by excluding ${#EXCLUDED_PALLETS[@]} from ${#ALL_PALLETS[@]}."

ERR_FILE="benchmarking_errors.txt"
# Delete the error file before each run.
rm -f $ERR_FILE

# Benchmark each pallet.
for PALLET in "${PALLETS[@]}"; do
  # If `-p` is used, skip benchmarks until the start pallet.
  if [ ! -z "$start_pallet" ] && [ "$start_pallet" != "$PALLET" ]
  then
    echo "[+] Skipping ${PALLET}..."
    continue
  else
    unset start_pallet
  fi

  output_file=""
  if [[ $PALLET == *"::"* ]]; then
    # translates e.g. "pallet_foo::bar" to "pallet_foo_bar"
    output_file="${PALLET//::/_}.rs"
  fi

  echo "[+] Benchmarking $PALLET with weight file $WEIGHT_FILE";

  OUTPUT=$(
    $SUBSTRATE benchmark pallet \
    --steps=50 \
    --repeat=20 \
    --pallet="$PALLET" \
    --extrinsic="*" \
    --wasm-execution=compiled \
    --heap-pages=4096 \
    --output="./runtime/src/weights/${output_file}" 2>&1
  )
  if [ $? -ne 0 ]; then
    echo "$OUTPUT" >> "$ERR_FILE"
    echo "[-] Failed to benchmark $PALLET. Error written to $ERR_FILE; continuing..."
  fi
done

# Update the block and extrinsic overhead weights.
echo "[+] Benchmarking block and extrinsic overheads..."
OUTPUT=$(
  $SUBSTRATE benchmark overhead \
  --wasm-execution=compiled \
  --weight-path="./runtime/src/weights/" \
  --warmup=10 \
  --repeat=100 2>&1
)
if [ $? -ne 0 ]; then
  echo "$OUTPUT" >> "$ERR_FILE"
  echo "[-] Failed to benchmark the block and extrinsic overheads. Error written to $ERR_FILE; continuing..."
fi

echo "[+] Benchmarking the machine..."
OUTPUT=$(
  $SUBSTRATE benchmark machine 2>&1
)
if [ $? -ne 0 ]; then
  # Do not write the error to the error file since it is not a benchmarking error.
  echo "[-] Failed the machine benchmark:\n$OUTPUT"
fi

# Check if the error file exists.
if [ -f "$ERR_FILE" ]; then
  echo "[-] Some benchmarks failed. See: $ERR_FILE"
  exit 1
else
  echo "[+] All benchmarks passed."
  exit 0
fi
