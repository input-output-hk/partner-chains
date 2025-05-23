#! /bin/sh
# This script updates JSON chain-spec file with the values according to present env variables.
# It updates values only if the env variable is present and the value in chain-spec is the default one.

if [ -z "$GOVERNED_MAP_VALIDATOR_ADDRESS" ]
then
    echo "GOVERNED_MAP_VALIDATOR_ADDRESS is not set. Not attempting to update chain-spec value for it."
else
    # -p for plain output, -c 128 to prevent wrapping lines
    export ADDRESS_HEX="0x$(echo $GOVERNED_MAP_VALIDATOR_ADDRESS | xxd -p -c 128)"
    jq --arg value $ADDRESS_HEX '.genesis.runtimeGenesis.config.governedMap.mainChainScripts.validator_address |= $value' $1 > chain-spec.json.tmp
    mv chain-spec.json.tmp $1
fi

if [ -z "$GOVERNED_MAP_POLICY_ID" ]
then
    echo "GOVERNED_MAP_POLICY_ID is not set. Not attempting to update chain-spec value for it."
else
    jq --arg value $GOVERNED_MAP_POLICY_ID '.genesis.runtimeGenesis.config.governedMap.mainChainScripts.asset_policy_id |= $value' $1 > chain-spec.json.tmp
    mv chain-spec.json.tmp $1
fi
