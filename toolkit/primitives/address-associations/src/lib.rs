#![cfg_attr(not(feature = "std"), no_std)]

use parity_scale_codec::Encode;
use sidechain_domain::{MainchainAddress, UtxoId};

#[derive(Debug, Clone, Encode)]
pub struct AddressAssociationSignedMessage<SCAddr> {
	pub mainchain_address: MainchainAddress,
	pub partnerchain_address: SCAddr,
	pub genesis_utxo: UtxoId,
}
