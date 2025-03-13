use self::mock::NativeTokenManagement;
use self::mock::{RuntimeOrigin, Test};
use crate::mock::mock_pallet;
use crate::mock::new_test_ext;
use crate::*;
use frame_support::traits::UnfilteredDispatchable;
use sp_native_token_management::InherentError;

mod create_inherent {
	use super::*;

	#[test]
	fn creates_inherent_from_inherent_data() {
		new_test_ext().execute_with(|| {
			let inherent_data = test_inherent_data(1001);

			let result = NativeTokenManagement::create_inherent(&inherent_data);

			assert_eq!(result, Some(Call::transfer_tokens { token_amount: 1001.into() }))
		})
	}
	#[test]
	fn skips_inherent_if_no_data() {
		new_test_ext().execute_with(|| {
			let inherent_data = InherentData::new();

			let result = NativeTokenManagement::create_inherent(&inherent_data);

			assert_eq!(result, None)
		})
	}
}

mod check_inherent {
	use super::*;

	#[test]
	fn pass_when_inherent_matches_data() {
		new_test_ext().execute_with(|| {
			let inherent_data = test_inherent_data(1002);
			let inherent = Call::transfer_tokens { token_amount: 1002.into() };

			let result = NativeTokenManagement::check_inherent(&inherent, &inherent_data);

			assert_eq!(result, Ok(()))
		})
	}
	#[test]
	fn fail_when_token_amount_mismatch() {
		new_test_ext().execute_with(|| {
			let inherent_data = test_inherent_data(1002);
			let inherent = Call::transfer_tokens { token_amount: 9999.into() };

			let result = NativeTokenManagement::check_inherent(&inherent, &inherent_data);

			assert_eq!(
				result,
				Err(InherentError::IncorrectTokenNumberTransfered(1002.into(), 9999.into()))
			)
		})
	}
	#[test]
	fn fail_when_unexpected_inherent() {
		new_test_ext().execute_with(|| {
			let inherent_data = InherentData::new();
			let inherent = Call::transfer_tokens { token_amount: 1002.into() };

			let result = NativeTokenManagement::check_inherent(&inherent, &inherent_data);

			assert_eq!(result, Err(InherentError::UnexpectedTokenTransferInherent(1002.into())));
		})
	}
}

mod is_inherent_required {
	use super::*;

	#[test]
	fn yes_when_nonzero_data_present() {
		new_test_ext().execute_with(|| {
			let inherent_data = test_inherent_data(1001);
			let error = NativeTokenManagement::is_inherent_required(&inherent_data)
				.expect("Check should successfully run.")
				.expect("Check should return an error object.");

			assert_eq!(error, InherentError::TokenTransferNotHandled(1001.into()));
		})
	}

	#[test]
	fn no_when_data_present_but_is_zero() {
		new_test_ext().execute_with(|| {
			let inherent_data = test_inherent_data(0);
			let error = NativeTokenManagement::is_inherent_required(&inherent_data)
				.expect("Check should successfully run.");

			assert_eq!(error, None);
		})
	}

	#[test]
	fn no_when_data_absent() {
		new_test_ext().execute_with(|| {
			let inherent_data = InherentData::new();
			let result = NativeTokenManagement::is_inherent_required(&inherent_data)
				.expect("Check should successfully run.");

			assert!(result.is_none());
		})
	}
}

mod inherent {
	use super::*;

	#[test]
	fn succeeds_and_calls_transfer_handler() {
		new_test_ext().execute_with(|| {
			assert_eq!(Initialized::<Test>::get(), false);
			let inherent: Call<Test> = Call::transfer_tokens { token_amount: 1000.into() };

			let _ = inherent.dispatch_bypass_filter(RuntimeOrigin::none()).unwrap();

			assert_eq!(mock_pallet::LastTokenTransfer::<Test>::get().unwrap().0, 1000);
			assert_eq!(Initialized::<Test>::get(), true);
		})
	}

	#[test]
	fn fails_when_not_root() {
		new_test_ext().execute_with(|| {
			let inherent: Call<Test> = Call::transfer_tokens { token_amount: 1000.into() };

			let result = inherent.dispatch_bypass_filter(RuntimeOrigin::signed(Default::default()));

			assert_eq!(result.unwrap_err().error, DispatchError::BadOrigin);

			assert_eq!(mock_pallet::LastTokenTransfer::<Test>::get(), None)
		})
	}
}

fn test_inherent_data(token_amount: u128) -> InherentData {
	let mut inherent_data = InherentData::new();
	inherent_data
		.put_data(
			INHERENT_IDENTIFIER,
			&TokenTransferData { token_amount: NativeTokenAmount(token_amount) },
		)
		.unwrap();
	inherent_data
}
