pub mod block_proposal;
pub mod inherent_digest;

pub use inherent_digest::InherentDigest;
use sp_consensus_slots::Slot;

/// Provides the current slot for Aura verification purpose.
pub trait CurrentSlotProvider {
	/// Returns the current slot, according to wall-time and slot duration configuration.
	fn slot(&self) -> Slot;
}
