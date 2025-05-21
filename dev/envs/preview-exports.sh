export CARDANO_SECURITY_PARAMETER=432
export CARDANO_ACTIVE_SLOTS_COEFF=0.05
# Timestamp for the MC_FIRST_EPOCH_NUMBER
# Genesis should not have a timestamp before this one, this should be divisible by both sidechain slot and epoch durations
export MC__FIRST_EPOCH_TIMESTAMP_MILLIS=1666656000000
# First Shelley epoch number on Cardano
export MC__FIRST_EPOCH_NUMBER=0
# Should be divisible by Sidechain epoch duration (which is SlotDuration * SlotsPerEpoch and those params can be found in runtime/src/lib.rs)
export MC__EPOCH_DURATION_MILLIS=86400000
# First Shelley slot number on Cardano
export MC__FIRST_SLOT_NUMBER=0
