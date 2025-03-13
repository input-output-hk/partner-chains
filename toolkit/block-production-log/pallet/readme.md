# Block Production Log pallet

This pallet keeps a sorted log of Slot to BlockProducerId.
It is a user decision what is the concrete type of BlockProducerId.
Inherent data provider that provides BlockProducerId should be wired into the node to make this pallet useful.
User should periodically call `take_prefix` to shrink used storage and consume the log according to the needs.
Pallet is ready for many blocks per slot.
