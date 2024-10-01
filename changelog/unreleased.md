## Changed

* Added 'deregister' command to partner-chains-cli.
* Made `MainChainScripts` in the native token pallet optional. If they are not set, the inherent data
provider will not query the main chain state or produce inherent data at all.
* ETCM-8366 - native token management pallet can now observe historical transfers when added after the genesis block

## Removed

## Fixed

## Added
* Added `new_for_runtime_version` factory for the native token inherent data provider,
allowing to selectively query main chain state based on runtime version
