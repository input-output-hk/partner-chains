#!/bin/bash

export GENESIS_UTXO="${GENESIS_UTXO:-0000000000000000000000000000000000000000000000000000000000000000#0}"

cargo build --locked --release

echo "Building chain-spec with parameters:"
echo "  Essential parameters:"
echo "      genesis utxo: $GENESIS_UTXO"
echo "  SessionValidatorManagement main chain configuration:"
echo "      committee_candidate_address: $COMMITTEE_CANDIDATE_ADDRESS"
echo "      d_parameter_policy_id: $D_PARAMETER_POLICY_ID"
echo "      permissioned_candidates_policy_id: $PERMISSIONED_CANDIDATES_POLICY_ID"

if [[ -n $CHAIN ]]; then
    export CHAIN_OPTION="--chain=${CHAIN}"
else
    export CHAIN_OPTION=""
fi

if [[ "$RAW" == "true" ]]; then
    export RAW_OPTION="--raw"
else
    export RAW_OPTION=""
fi

cargo run --locked --release --bin partner-chains-node -- build-spec ${CHAIN_OPTION} --disable-default-bootnode ${RAW_OPTION} > chain-spec.json

# Change the owner of chain-spec.json if the environment variable is set
if [[ -n $SPEC_FILE_UID ]]; then
    chown $SPEC_FILE_UID chain-spec.json
fi
# Change the group of chain-spec.json if the environment variable is set (note colon in the command).
if [[ -n $SPEC_FILE_GID ]]; then
    chown :$SPEC_FILE_GID chain-spec.json
fi

/bin/bash
