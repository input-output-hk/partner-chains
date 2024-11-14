# Main chain follower CLI

This is a thin CLI wrapper around the main chain follower API.

## Usage

```sh
cargo run --bin main-chain-follower-cli -- <request> <arguments*>
# or
./target/debug/main-chain-follower-cli <request> <arguments*>
```
Example:
```
cargo run --bin main-chain-follower-cli -- get-all-incoming-transactions 13 c3beb3ea4a0f7ed44bcad916e96838eaa0db5a6e29395c5beb4ea0b036a3c243
```

Run with `--help` for complete usage.

## Configuration

db-sync-follower is used beneath, so same set of env variables to configure this CLI as is required for db-sync-follower.

For devnet: the easiest way to set them is by enabling `direnv` in the repo or sourcing `.envrc` directly.
