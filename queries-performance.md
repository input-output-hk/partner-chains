# Mainnet queries performance analysis

## Block queries

Important context is that node queries adjacent or overlapping blocks ranges,
so even if the first query takes few milliseconds,
the next one needs fraction of millisecond to complete.

### get_latest_block_info

Consistent results:
Planning Time: 0.095 ms
Execution Time: 0.038 ms

### get_blocks_by_numbers

For range of 1000 blocks, results varying but execution times are sub 10ms.
This query is used during sync from genesis and it should not be a factor in performance,
because it allows to get blocks from over 5 hours in less than 10 ms.

### get_highest_block

This query is used to get the highest block with additional constraints about its timestamps.
The timestamps range on mainnet is 2*k/active_slots_coefficient, so it is 24 hours.
Slots are indexed and they support the query efficiency.

Execution time on mainnet is usually under 0.2.

### get_block_by_hash

Execution time is under 0.2 ms.

### get_latest_block_for_epoch

This query performs backwards full scan, so it is indeed slow when then querying the old epochs, what happens when syncing chain from genesis.
Therefore this query should be optimized by calculating the max slot for given epoch and using this slot additionally (or exclusively) in the query.

## Candidates related queries

### get_latest_stable_epoch

Execution time under 0.2 ms, usually around 0.05 ms.

### get_stake_distribution

Execution time is nearly 1 second. Results of the query are cached.
Risk here is that the first invocation to fill in the cache takes a lot of time,
and it can jeopardize production of the first block that needs this data.
Optimization would be to make this request a few seconds (a block time) before the result is needed.

### get_token_utxo_for_epoch

Without optimization the query took 130 ms when executed for epoch 400. Subsequent queries for the same token where considerably faster.
It seems that the query could be optimized by adding the origin block slot_no to the query conditions to use index,
but it seems not really required.

Policy id used: 29f2fdede501f7e7ee301c6c5db5162dae51d31cc63424177a107f0e (Cutémon by Night Parade of 100 Demons)
Asset name: 437574656d6f6e31333235 (Cutemon1325)

### get_epoch_nonce

Under 0.2 ms, usually under 0.1 ms.

### get_utxos_for_address

This query took 43 seconds when executed for the first time, further invocations, for different time range were way faster, around 1.5 seconds.
This need some more investigation, possibly lets look how cexplorer or other chain indexers are able to quickly display UTXOs a given address.

## Native token observation

### get_total_native_tokens_transfered

The first query took 17 seconds, but the subsequent ones were under 1 second.
This query should be executed only once in the partner chains lifetime.
From version v1.4 we could optimize the query, by limiting the time range to the spending of genesis utxo?

### get_native_token_transfers

This query is called for a narrow margin of blocks, it is very fast. No worries here.

## Conclusions

0. Access patterns matter to real times, so I recommend we setup one partner chain instance at the main chain and start a partner-chains node to observe it.
We will get the data from its metrics endpoint, more thrustworthy then manually executed queries.
1. get_latest_block_for_epoch should be optimized by adding the slot condition to the query
2. Candidates related queries could be executed in a non-blocking way a few seconds before the result is need to fill the cache
3. Queries for registrations could be re-implemented.
4. The initial native tokens query, the first one to query from genesis could cause the node to be not able to produce blocks for some time after the node startup.
5. `idx_ma_tx_out_ident` takes many minutes to be created. If we delay this action to when the PC node is started for a first time,
it will cause that from a user perspective, the node has frozen. Some way to mitigate this problem should be investigated.
