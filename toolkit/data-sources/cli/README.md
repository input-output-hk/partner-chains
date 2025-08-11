# Partner Chains Data Sources CLI

This crate provides a thin CLI wrapper around the db-sync implementation of the Partner Chains data source APIs.

## Usage

Data sources are configured with the toolkit code, that uses env variables.
For configuration details refer to [docs](../../../docs/intro.md#data-source-configuration)

Example:
```sh
# Postgres database location
export CARDANO_DATA_SOURCE="db-sync"
export DB_SYNC_POSTGRES_CONNECTION_STRING="postgres://postgres:password123@localhost/cexplorer"
# Required configuration parameter. Use 0.
export BLOCK_STABILITY_MARGIN=0
# Cardano parameters required for proper computation of epochs and block stability
export CARDANO_SECURITY_PARAMETER=432
export CARDANO_ACTIVE_SLOTS_COEFF=0.05
export MC__FIRST_EPOCH_TIMESTAMP_MILLIS=1666656000000
export MC__FIRST_EPOCH_NUMBER=0
export MC__EPOCH_DURATION_MILLIS=86400000
export MC__FIRST_SLOT_NUMBER=0
```

```sh
cargo run --bin partner-chains-data-sources-cli -- <request> <arguments*>
# or
./target/debug/partner-chains-data-sources-cli <request> <arguments*>
```
Example:
```
cargo run --bin partner-chains-data-sources-cli -- get-stable-block-for 0x37286c32f2a9e7fd037b459bf316242127209debbfe467d876f452e4b46ab763 1748423154000
...
{
  "number": 3277100,
  "hash": "0x37286c32f2a9e7fd037b459bf316242127209debbfe467d876f452e4b46ab763",
  "epoch": 946,
  "slot": 81757924,
  "timestamp": 1748413924
}
```

Run with `--help` for complete usage.
