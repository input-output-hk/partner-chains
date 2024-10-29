Perform the HF:

cardano-cli conway governance action create-hardfork \
  --testnet \
  --governance-action-deposit 100000000 \
  --deposit-return-stake-verification-key-file /configurations/cardano/cardano-node-1/keys/owner-stake.vkey \
  --anchor-url "http://www.hardfork.com" \
  --anchor-data-hash d72259550f75d7478f1840480136e1cf2f48ce214f3847bb4aa37b7bb7bd8e7f \
  --protocol-major-version 10 \
  --protocol-minor-version 0 \
  --out-file test_hardfork_ci0_oqn_hardfork.action

cardano-cli conway transaction build \
  --tx-in "1dec8eaf25c6ec1371282f23fa7aaa46ceb914a1d6c4dbb9258031711f2fb83b#0" \
  --proposal-file test_hardfork_ci0_oqn_hardfork.action \
  --change-address $(cat /configurations/cardano/cardano-node-1/keys/owner.addr) \
  --witness-override 1 \
  --out-file test_hardfork_ci0_oqn_action_tx.body \
  --testnet-magic 42

cardano-cli conway transaction sign \
  --tx-body-file test_hardfork_ci0_oqn_action_tx.body \
  --testnet-magic 42 \
  --signing-key-file /configurations/cardano/cardano-node-1/keys/owner-utxo.skey \
  --out-file test_hardfork_ci0_oqn_action_tx.signed

cardano-cli conway transaction submit \
  --testnet-magic 42 \
  --tx-file test_hardfork_ci0_oqn_action_tx.signed

#############################################################################

Get governance action tx id and index:

cardano-cli conway query gov-state --testnet-magic 42

...
    "proposals": [
        {
            "actionId": {
                "govActionIx": 0,
                "txId": "fa4b646eee46e961573dba5d2cd487920f7d69e751da0e9064603c7ba0fcb75b"
            },
...
#############################################################################
Vote for the proposal to get accepted:

####### Constituional Committee votes:

cardano-cli conway governance vote create --yes \
  --governance-action-tx-id fa4b646eee46e961573dba5d2cd487920f7d69e751da0e9064603c7ba0fcb75b \
  --governance-action-index 0 \
  --cc-hot-verification-key-file /configurations/cardano/cc-member-1/cc_member1_committee_hot.vkey \
  --anchor-url "http://www.cc-vote1.com" \
  --anchor-data-hash 5d372dca1a4cc90d7d16d966c48270e33e3aa0abcb0e78f0d5ca7ff330d2245d \
  --out-file test_hardfork_ci0_oqn_yes_cc1_cc.vote

cardano-cli conway governance vote create --yes \
  --governance-action-tx-id fa4b646eee46e961573dba5d2cd487920f7d69e751da0e9064603c7ba0fcb75b \
  --governance-action-index 0 \
  --cc-hot-verification-key-file /configurations/cardano/cc-member-2/cc_member2_committee_hot.vkey \
  --anchor-url "http://www.cc-vote1.com" \
  --anchor-data-hash 5d372dca1a4cc90d7d16d966c48270e33e3aa0abcb0e78f0d5ca7ff330d2245d \
  --out-file test_hardfork_ci0_oqn_yes_cc2_cc.vote

cardano-cli conway governance vote create --yes \
  --governance-action-tx-id fa4b646eee46e961573dba5d2cd487920f7d69e751da0e9064603c7ba0fcb75b \
  --governance-action-index 0 \
  --cc-hot-verification-key-file /configurations/cardano/cc-member-3/cc_member3_committee_hot.vkey \
  --anchor-url "http://www.cc-vote1.com" \
  --anchor-data-hash 5d372dca1a4cc90d7d16d966c48270e33e3aa0abcb0e78f0d5ca7ff330d2245d \
  --out-file test_hardfork_ci0_oqn_yes_cc3_cc.vote

####### SPO votes:

cardano-cli conway governance vote create --yes \
  --governance-action-tx-id fa4b646eee46e961573dba5d2cd487920f7d69e751da0e9064603c7ba0fcb75b \
  --governance-action-index 0 \
  --cold-verification-key-file /configurations/cardano/cardano-node-1/cold.vkey \
  --anchor-url "http://www.spo-vote1.com" \
  --anchor-data-hash 5d372dca1a4cc90d7d16d966c48270e33e3aa0abcb0e78f0d5ca7ff330d2245d \
  --out-file test_hardfork_ci0_oqn_yes_pool1_spo.vote

cardano-cli conway governance vote create --yes \
  --governance-action-tx-id fa4b646eee46e961573dba5d2cd487920f7d69e751da0e9064603c7ba0fcb75b \
  --governance-action-index 0 \
  --cold-verification-key-file /configurations/cardano/cardano-node-2/cold.vkey \
  --anchor-url "http://www.spo-vote1.com" \
  --anchor-data-hash 5d372dca1a4cc90d7d16d966c48270e33e3aa0abcb0e78f0d5ca7ff330d2245d \
  --out-file test_hardfork_ci0_oqn_yes_pool2_spo.vote

cardano-cli conway governance vote create --yes \
  --governance-action-tx-id fa4b646eee46e961573dba5d2cd487920f7d69e751da0e9064603c7ba0fcb75b \
  --governance-action-index 0 \
  --cold-verification-key-file /configurations/cardano/cardano-node-3/cold.vkey \
  --anchor-url "http://www.spo-vote1.com" \
  --anchor-data-hash 5d372dca1a4cc90d7d16d966c48270e33e3aa0abcb0e78f0d5ca7ff330d2245d \
  --out-file test_hardfork_ci0_oqn_yes_pool3_spo.vote

cardano-cli conway transaction build \
  --tx-in "fa4b646eee46e961573dba5d2cd487920f7d69e751da0e9064603c7ba0fcb75b#0" \
  --vote-file test_hardfork_ci0_oqn_yes_cc1_cc.vote \
  --vote-file test_hardfork_ci0_oqn_yes_cc2_cc.vote \
  --vote-file test_hardfork_ci0_oqn_yes_cc3_cc.vote \
  --vote-file test_hardfork_ci0_oqn_yes_pool1_spo.vote \
  --vote-file test_hardfork_ci0_oqn_yes_pool2_spo.vote \
  --vote-file test_hardfork_ci0_oqn_yes_pool3_spo.vote \
  --change-address $(cat /configurations/cardano/cardano-node-1/keys/owner.addr) \
  --witness-override 3 \
  --out-file test_hardfork_ci0_oqn_yes_vote_tx.body \
  --testnet-magic 42

cardano-cli conway transaction sign \
  --tx-body-file test_hardfork_ci0_oqn_yes_vote_tx.body \
  --testnet-magic 42 \
  --signing-key-file /configurations/cardano/cardano-node-1/keys/owner-utxo.skey \
  --signing-key-file /configurations/cardano/cc-member-1/cc_member1_committee_hot.skey \
  --signing-key-file /configurations/cardano/cc-member-2/cc_member2_committee_hot.skey \
  --signing-key-file /configurations/cardano/cc-member-3/cc_member3_committee_hot.skey \
  --signing-key-file /configurations/cardano/cardano-node-1/cold.skey \
  --signing-key-file /configurations/cardano/cardano-node-2/cold.skey \
  --signing-key-file /configurations/cardano/cardano-node-3/cold.skey \
  --out-file test_hardfork_ci0_oqn_yes_vote_tx.signed

cardano-cli conway transaction submit \
  --testnet-magic 42 \
  --tx-file test_hardfork_ci0_oqn_yes_vote_tx.signed

######################################
Wait 2 epochs and verify the change:
cardano-cli conway query protocol-parameters --testnet-magic 42 | jq .protocolVersion