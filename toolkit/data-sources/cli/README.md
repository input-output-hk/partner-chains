# Partner Chains Data Sources CLI

This is a thin CLI wrapper around the db-sync implementation of the Partner Chains data source APIs.

## Usage

```sh
cargo run --bin partner-chains-data-sources-cli -- <request> <arguments*>
# or
./target/debug/partner-chains-data-sources-cli <request> <arguments*>
```
Example:
```
cargo run --bin partner-chains-data-sources-cli -- get-all-incoming-transactions 13 c3beb3ea4a0f7ed44bcad916e96838eaa0db5a6e29395c5beb4ea0b036a3c243
```

Run with `--help` for complete usage.

## Configuration

partner-chains-db-sync-data-sources is used beneath, so same set of env variables to configure this CLI as is required for partner-chains-db-sync-data-sources.

For devnet: the easiest way to set them is by enabling `direnv` in the repo or sourcing `.envrc` directly.
