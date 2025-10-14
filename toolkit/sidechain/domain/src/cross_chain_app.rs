//! Substrate key pair types for Partner Chain cross-chain keys

use crate::SidechainPublicKey;
use parity_scale_codec::MaxEncodedLen;
use sp_core::crypto::AccountId32;
use sp_runtime::app_crypto::{app_crypto, ecdsa};
use sp_runtime::traits::IdentifyAccount;
use sp_runtime::{KeyTypeId, MultiSigner};
use sp_std::vec::Vec;

/// Key type for the cross-chain keys
pub const KEY_TYPE_ID: KeyTypeId = KeyTypeId(*b"crch");

app_crypto!(ecdsa, KEY_TYPE_ID);

impl MaxEncodedLen for Signature {
	fn max_encoded_len() -> usize {
		ecdsa::Signature::max_encoded_len()
	}
}

impl From<Signature> for Vec<u8> {
	fn from(value: Signature) -> Self {
		value.into_inner().0.to_vec()
	}
}

impl From<Public> for AccountId32 {
	fn from(value: Public) -> Self {
		MultiSigner::from(ecdsa::Public::from(value)).into_account()
	}
}

impl From<Public> for Vec<u8> {
	fn from(value: Public) -> Self {
		value.into_inner().0.to_vec()
	}
}

impl TryFrom<SidechainPublicKey> for Public {
	type Error = SidechainPublicKey;
	fn try_from(pubkey: SidechainPublicKey) -> Result<Self, Self::Error> {
		Public::try_from(pubkey.0.as_slice()).map_err(|_| pubkey)
	}
}
