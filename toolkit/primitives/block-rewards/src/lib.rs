#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod test;

use parity_scale_codec::Encode;
use sp_inherents::{InherentIdentifier, IsFatalError};
use sp_runtime::traits::One;

#[cfg(feature = "std")]
use {parity_scale_codec::Decode, sp_core::bytes::FromHexError, sp_inherents::InherentData};

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"beneficr";

#[derive(Encode, PartialEq)]
#[cfg_attr(not(feature = "std"), derive(Debug))]
#[cfg_attr(feature = "std", derive(Decode, thiserror::Error, sp_runtime::RuntimeDebug))]
pub enum InherentError {
	#[cfg_attr(feature = "std", error("Block beneficiary inherent must be produced every block"))]
	InherentRequired,
}
impl IsFatalError for InherentError {
	fn is_fatal_error(&self) -> bool {
		true
	}
}

/// Calculates block reward for the current block
pub trait GetBlockRewardPoints<Reward> {
	fn get_block_reward() -> Reward;
}

/// Simple strategy rewarding equal units per each block
pub struct SimpleBlockCount;
impl<Reward: One> GetBlockRewardPoints<Reward> for SimpleBlockCount {
	fn get_block_reward() -> Reward {
		Reward::one()
	}
}

#[cfg(feature = "std")]
#[derive(Debug)]
pub struct BlockBeneficiaryInherentProvider<BeneficiaryId> {
	pub beneficiary_id: BeneficiaryId,
}

#[cfg(feature = "std")]
#[derive(thiserror::Error, sp_runtime::RuntimeDebug)]
pub enum InherentProviderCreationError {
	#[error("Block beneficiary is not valid hex: {0}")]
	InvalidHex(FromHexError),
	#[error("Invalid block beneficiary bytes: {0:?}")]
	InvalidBeneficiary(Vec<u8>),
	#[error("Block beneficiary environment variable {0} not set")]
	NotSet(String),
}

#[cfg(feature = "std")]
impl<BeneficiaryId> BlockBeneficiaryInherentProvider<BeneficiaryId>
where
	BeneficiaryId: TryFrom<Vec<u8>> + Send + Sync + Encode,
	<BeneficiaryId as TryFrom<Vec<u8>>>::Error: std::fmt::Debug,
{
	pub fn from_env(env_var: &str) -> Result<Self, InherentProviderCreationError> {
		let beneficiary_string = std::env::var(env_var)
			.map_err(|_| InherentProviderCreationError::NotSet(env_var.into()))?;
		let beneficiary_bytes = sp_core::bytes::from_hex(&beneficiary_string)
			.map_err(InherentProviderCreationError::InvalidHex)?;
		let beneficiary_id = BeneficiaryId::try_from(beneficiary_bytes.clone())
			.map_err(|_| InherentProviderCreationError::InvalidBeneficiary(beneficiary_bytes))?;

		Ok(BlockBeneficiaryInherentProvider { beneficiary_id })
	}
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl<BeneficiaryId> sp_inherents::InherentDataProvider
	for BlockBeneficiaryInherentProvider<BeneficiaryId>
where
	BeneficiaryId: TryFrom<Vec<u8>> + Send + Sync + Encode,
{
	async fn provide_inherent_data(
		&self,
		inherent_data: &mut InherentData,
	) -> Result<(), sp_inherents::Error> {
		inherent_data.put_data(INHERENT_IDENTIFIER, &self.beneficiary_id)
	}

	async fn try_handle_error(
		&self,
		identifier: &InherentIdentifier,
		mut error: &[u8],
	) -> Option<Result<(), sp_inherents::Error>> {
		if *identifier == INHERENT_IDENTIFIER {
			let error = InherentError::decode(&mut error).ok()?;
			Some(Err(sp_inherents::Error::Application(Box::from(error))))
		} else {
			None
		}
	}
}
