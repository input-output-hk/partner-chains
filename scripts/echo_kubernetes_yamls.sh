# Script uses current env variables and outputs configuration to paste sidechain-infra-priv files.
cat >&1 << EOL
  committee_candidate_address: "$COMMITTEE_CANDIDATE_ADDRESS"
  d_parameter_policy_id: "$D_PARAMETER_POLICY_ID"
  permissioned_candidates_policy_id: "$PERMISSIONED_CANDIDATES_POLICY_ID"
EOL
