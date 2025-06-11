#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
	use scale_info::prelude::boxed::Box;
	use sp_runtime::Vec;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_session::Config {
		/// The caller origin, overarching type of all pallets origins.
		type PalletsOrigin: Parameter +
			Into<<Self as frame_system::Config>::RuntimeOrigin> +
			IsType<<<Self as frame_system::Config>::RuntimeOrigin as frame_support::traits::OriginTrait>::PalletsOrigin>;
	}

	impl<T: Config> Pallet<T> {
		/// Invokes the `pallet_session::Call::set_keys` function to set the session keys.
		///
		/// Allows the caller to set the session keys for the next session for particular user.
		///
		/// ## Complexity
		/// - `O(1)`. Actual cost depends on the number of length of `T::Keys::key_ids()` which is
		///   fixed.
		pub fn set_keys(
			as_origin: Box<T::PalletsOrigin>,
			keys: <T as pallet_session::Config>::Keys,
			proof: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			let call = pallet_session::Call::<T>::set_keys { keys, proof };

			use frame_support::traits::UnfilteredDispatchable;
			call.dispatch_bypass_filter((*as_origin).into())
		}
	}
}
