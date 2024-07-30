use frame_support::{Deserialize, Serialize};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::{ed25519, sr25519};

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Encode,
	Decode,
	TypeInfo,
	MaxEncodedLen,
	Serialize,
	Deserialize,
)]
pub struct SessionKeys {
	pub aura: [u8; 32],
	pub grandpa: [u8; 32],
}

impl From<(sr25519::Public, ed25519::Public)> for SessionKeys {
	fn from((aura, grandpa): (sr25519::Public, ed25519::Public)) -> Self {
		Self { aura: aura.0, grandpa: grandpa.0 }
	}
}

#[cfg(feature = "std")]
impl SessionKeys {
	pub fn from_seed(seed: &str) -> SessionKeys {
		let mut aura = format!("aura-{seed}").into_bytes();
		aura.resize(32, 0);
		let mut grandpa = format!("grandpa-{seed}").into_bytes();
		grandpa.resize(32, 0);
		SessionKeys { aura: aura.try_into().unwrap(), grandpa: grandpa.try_into().unwrap() }
	}
}
