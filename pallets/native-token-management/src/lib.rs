#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::inherent::IsFatalError;
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
pub use pallet::*;
use scale_info::prelude::fmt::Debug;
use sp_native_token_management::*;

#[derive(Encode)]
pub enum InherentError {
	TokenReleaseNotHandled,
	IncorrectTokenNumberReleased,
	UnexpectedTokenReleaseInherent,
}

impl IsFatalError for InherentError {
	fn is_fatal_error(&self) -> bool {
		true
	}
}

pub trait TokenReleaseHandler<TokenAmount> {
	fn handle_token_release(token_amount: TokenAmount) -> DispatchResult;
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type TokenAmount: TypeInfo + Encode + Decode + Clone + Debug + Eq;

		type TokenReleaseHandler: TokenReleaseHandler<Self::TokenAmount>;
	}

	#[pallet::event]
	pub enum Event<T: Config> {
		TokensReleased(T::TokenAmount),
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = InherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			Self::get_released_tokens_from_inherent_data(data)
				.map(|data| Call::release_tokens { token_amount: data.token_amount })
		}

		fn check_inherent(call: &Self::Call, data: &InherentData) -> Result<(), Self::Error> {
			let Some(token_release) = Self::get_released_tokens_from_inherent_data(data) else {
				return Ok(());
			};
			let Call::release_tokens { token_amount } = call else {
				return Err(InherentError::UnexpectedTokenReleaseInherent);
			};

			if token_release.token_amount == *token_amount {
				Ok(())
			} else {
				Err(InherentError::IncorrectTokenNumberReleased)
			}
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::release_tokens { .. })
		}

		fn is_inherent_required(data: &InherentData) -> Result<Option<Self::Error>, Self::Error> {
			Ok(Self::get_released_tokens_from_inherent_data(data)
				.map(|_| InherentError::TokenReleaseNotHandled))
		}
	}

	impl<T: Config> Pallet<T> {
		fn get_released_tokens_from_inherent_data(
			data: &InherentData,
		) -> Option<TokenReleaseData<T::TokenAmount>> {
			data.get_data::<TokenReleaseData<T::TokenAmount>>(&INHERENT_IDENTIFIER)
				.expect("Token release data not correctly encoded")
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight((0, DispatchClass::Mandatory))]
		pub fn release_tokens(
			origin: OriginFor<T>,
			token_amount: T::TokenAmount,
		) -> DispatchResult {
			ensure_none(origin)?;
			T::TokenReleaseHandler::handle_token_release(token_amount)
		}
	}
}
