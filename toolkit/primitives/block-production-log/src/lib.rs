#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod test;

use parity_scale_codec::Encode;
use sp_inherents::{InherentIdentifier, IsFatalError};
#[cfg(feature = "std")]
use {parity_scale_codec::Decode, sp_inherents::InherentData};

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"blprdlog";

#[derive(Encode, PartialEq)]
#[cfg_attr(not(feature = "std"), derive(Debug))]
#[cfg_attr(feature = "std", derive(Decode, thiserror::Error, sp_runtime::RuntimeDebug))]
pub enum InherentError {
	#[cfg_attr(feature = "std", error("Block Producer Id inherent must be provided every block"))]
	InherentRequired,
}
impl IsFatalError for InherentError {
	fn is_fatal_error(&self) -> bool {
		true
	}
}

#[cfg(feature = "std")]
#[derive(Debug)]
pub struct BlockProducerIdInherentProvider<T> {
	pub id: T,
}

#[cfg(feature = "std")]
impl<T> BlockProducerIdInherentProvider<T>
where
	T: TryFrom<Vec<u8>> + Send + Sync + Encode,
	<T as TryFrom<Vec<u8>>>::Error: std::fmt::Debug,
{
	pub fn from_env(env_var: &str) -> Result<Self, String> {
		let env_var_value = std::env::var(env_var).map_err(|_| {
			format!("Block Producer Id environment variable '{env_var}' is not set")
		})?;
		let bytes = sp_core::bytes::from_hex(&env_var_value)
			.map_err(|_| format!("Block Producer Id environment variable '{env_var}' value '{env_var_value}' is not a valid hex string"))?;
		let id = T::try_from(bytes.clone()).map_err(|e| {
			format!("Could not convert '{env_var_value}' into Block Producer Id. Cause: {e:#?}")
		})?;

		Ok(BlockProducerIdInherentProvider { id })
	}
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl<T> sp_inherents::InherentDataProvider for BlockProducerIdInherentProvider<T>
where
	T: TryFrom<Vec<u8>> + Send + Sync + Encode,
{
	async fn provide_inherent_data(
		&self,
		inherent_data: &mut InherentData,
	) -> Result<(), sp_inherents::Error> {
		inherent_data.put_data(INHERENT_IDENTIFIER, &self.id)
	}

	async fn try_handle_error(
		&self,
		identifier: &InherentIdentifier,
		mut error: &[u8],
	) -> Option<Result<(), sp_inherents::Error>> {
		if identifier == &INHERENT_IDENTIFIER {
			let error = InherentError::decode(&mut error).ok()?;
			Some(Err(sp_inherents::Error::Application(Box::from(error))))
		} else {
			None
		}
	}
}
