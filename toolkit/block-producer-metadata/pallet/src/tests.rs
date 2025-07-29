use crate::HoldReason;

use super::*;
use crate::mock::mock_pallet::CurrentTime;
use frame_support::{assert_noop, assert_ok, traits::tokens::fungible::InspectHold};
use frame_system::pallet_prelude::OriginFor;
use mock::*;
use sidechain_domain::byte_string::SizedByteString;

mod upsert_metadata {

	use super::*;
	use pretty_assertions::assert_eq;
	#[test]
	fn saves_new_metadata_and_holds_fee() {
		new_test_ext().execute_with(|| {
			let initial_balance = Balances::free_balance(&FUNDED_ACCOUNT);

			assert_ok!(super::Pallet::<Test>::upsert_metadata(
				OriginFor::<Test>::signed(FUNDED_ACCOUNT),
				url_metadata_1(),
				cross_chain_signature(FUNDED_ACCOUNT, Some(url_metadata_1())),
				cross_chain_pub_key(),
				valid_before()
			));

			assert_eq!(
				Pallet::<Test>::get_metadata_for(&cross_chain_pub_key()),
				Some(url_metadata_1())
			);

			let final_balance = Balances::free_balance(&FUNDED_ACCOUNT);
			assert_eq!(final_balance, initial_balance - MetadataHoldAmount::get());

			// Check that the amount is held, not burned
			let held_balance = Balances::balance_on_hold(
				&RuntimeHoldReason::BlockProducerMetadata(HoldReason::MetadataDeposit),
				&FUNDED_ACCOUNT,
			);
			assert_eq!(held_balance, MetadataHoldAmount::get());
		})
	}

	#[test]
	fn updates_metadata_without_holding_additional_fee() {
		new_test_ext().execute_with(|| {
			assert_ok!(super::Pallet::<Test>::upsert_metadata(
				OriginFor::<Test>::signed(FUNDED_ACCOUNT),
				url_metadata_1(),
				cross_chain_signature(FUNDED_ACCOUNT, Some(url_metadata_1())),
				cross_chain_pub_key(),
				valid_before()
			));

			assert_eq!(
				Pallet::<Test>::get_metadata_for(&cross_chain_pub_key()),
				Some(url_metadata_1())
			);

			assert_ok!(super::Pallet::<Test>::upsert_metadata(
				OriginFor::<Test>::signed(FUNDED_ACCOUNT),
				url_metadata_2(),
				cross_chain_signature(FUNDED_ACCOUNT, Some(url_metadata_2())),
				cross_chain_pub_key(),
				valid_before()
			));

			assert_eq!(
				Pallet::<Test>::get_metadata_for(&cross_chain_pub_key()),
				Some(url_metadata_2())
			);

			let account_1_balance = Balances::free_balance(&FUNDED_ACCOUNT);
			let account_1_held = Balances::balance_on_hold(
				&RuntimeHoldReason::BlockProducerMetadata(HoldReason::MetadataDeposit),
				&FUNDED_ACCOUNT,
			);

			let account_2_balance = Balances::free_balance(&FUNDED_ACCOUNT_2);
			let account_2_held = Balances::balance_on_hold(
				&RuntimeHoldReason::BlockProducerMetadata(HoldReason::MetadataDeposit),
				&FUNDED_ACCOUNT_2,
			);

			assert_eq!(account_1_balance, INITIAL_BALANCE - MetadataHoldAmount::get());
			assert_eq!(account_1_held, MetadataHoldAmount::get());

			assert_eq!(account_2_balance, INITIAL_BALANCE);
			assert_eq!(account_2_held, 0);
		})
	}

	#[test]
	fn rejects_invalid_signature() {
		new_test_ext().execute_with(|| {
			assert_eq!(
				super::Pallet::<Test>::upsert_metadata(
					OriginFor::<Test>::signed(FUNDED_ACCOUNT),
					url_metadata_2(),
					cross_chain_signature(FUNDED_ACCOUNT, Some(url_metadata_1())),
					cross_chain_pub_key(),
					valid_before()
				)
				.unwrap_err(),
				Error::<Test>::InvalidMainchainSignature.into()
			);
		})
	}

	#[test]
	fn fails_with_insufficient_balance() {
		new_test_ext().execute_with(|| {
			assert_noop!(
				super::Pallet::<Test>::upsert_metadata(
					OriginFor::<Test>::signed(POOR_ACCOUNT),
					url_metadata_1(),
					cross_chain_signature(POOR_ACCOUNT, Some(url_metadata_1())),
					cross_chain_pub_key(),
					valid_before()
				),
				Error::<Test>::InsufficientBalance
			);
		})
	}

	#[test]
	fn rejects_non_owner_origin() {
		new_test_ext().execute_with(|| {
			super::Pallet::<Test>::upsert_metadata(
				OriginFor::<Test>::signed(FUNDED_ACCOUNT),
				url_metadata_1(),
				cross_chain_signature(FUNDED_ACCOUNT, Some(url_metadata_1())),
				cross_chain_pub_key(),
				valid_before(),
			)
			.expect("First insert should succeed");

			let error = super::Pallet::<Test>::upsert_metadata(
				OriginFor::<Test>::signed(FUNDED_ACCOUNT_2),
				url_metadata_1(),
				cross_chain_signature(FUNDED_ACCOUNT_2, Some(url_metadata_1())),
				cross_chain_pub_key(),
				valid_before(),
			)
			.unwrap_err();
			assert_eq!(error, Error::<Test>::NotTheOwner.into());
		})
	}

	#[test]
	fn rejects_signature_past_validity_time() {
		new_test_ext().execute_with(|| {
			CurrentTime::<Test>::set(valid_before() + 1);

			let error = super::Pallet::<Test>::upsert_metadata(
				OriginFor::<Test>::signed(FUNDED_ACCOUNT),
				url_metadata_1(),
				cross_chain_signature(FUNDED_ACCOUNT, Some(url_metadata_1())),
				cross_chain_pub_key(),
				valid_before(),
			)
			.unwrap_err();

			assert_eq!(error, Error::<Test>::PastValidityTime.into());
		})
	}
}

mod delete_metadata {
	use super::*;
	use pretty_assertions::assert_eq;

	#[test]
	fn deletes_metadata_and_returns_fee() {
		new_test_ext().execute_with(|| {
			assert_ok!(super::Pallet::<Test>::upsert_metadata(
				OriginFor::<Test>::signed(FUNDED_ACCOUNT),
				url_metadata_1(),
				cross_chain_signature(FUNDED_ACCOUNT, Some(url_metadata_1())),
				cross_chain_pub_key(),
				valid_before()
			));

			assert_eq!(
				Pallet::<Test>::get_metadata_for(&cross_chain_pub_key()),
				Some(url_metadata_1())
			);

			assert_ok!(super::Pallet::<Test>::delete_metadata(
				OriginFor::<Test>::signed(FUNDED_ACCOUNT),
				cross_chain_pub_key(),
				cross_chain_signature(FUNDED_ACCOUNT, None),
				valid_before()
			));

			assert_eq!(Pallet::<Test>::get_metadata_for(&cross_chain_pub_key()), None);

			let account_1_balance = Balances::free_balance(&FUNDED_ACCOUNT);
			let account_1_held =
				Balances::balance_on_hold(&HoldReason::MetadataDeposit.into(), &FUNDED_ACCOUNT);

			assert_eq!(account_1_balance, INITIAL_BALANCE);
			assert_eq!(account_1_held, 0);
		})
	}

	#[test]
	fn rejects_non_owner_origin() {
		new_test_ext().execute_with(|| {
			super::Pallet::<Test>::upsert_metadata(
				OriginFor::<Test>::signed(FUNDED_ACCOUNT),
				url_metadata_1(),
				cross_chain_signature(FUNDED_ACCOUNT, Some(url_metadata_1())),
				cross_chain_pub_key(),
				valid_before(),
			)
			.expect("First insert should succeed");

			let error = super::Pallet::<Test>::delete_metadata(
				OriginFor::<Test>::signed(FUNDED_ACCOUNT_2),
				cross_chain_pub_key(),
				cross_chain_signature(FUNDED_ACCOUNT_2, None),
				valid_before(),
			)
			.unwrap_err();
			assert_eq!(error, Error::<Test>::NotTheOwner.into());
		})
	}

	#[test]
	fn rejects_signature_past_validity_time() {
		new_test_ext().execute_with(|| {
			CurrentTime::<Test>::set(valid_before() + 1);

			let error = super::Pallet::<Test>::delete_metadata(
				OriginFor::<Test>::signed(FUNDED_ACCOUNT),
				cross_chain_pub_key(),
				cross_chain_signature(FUNDED_ACCOUNT, None),
				valid_before(),
			)
			.unwrap_err();

			assert_eq!(error, Error::<Test>::PastValidityTime.into());
		})
	}
}

fn url_metadata_1() -> BlockProducerUrlMetadata {
	BlockProducerUrlMetadata {
		url: "https://cool.stuff/spo.json".try_into().unwrap(),
		hash: SizedByteString::from([0; 32]),
	}
}

fn url_metadata_2() -> BlockProducerUrlMetadata {
	BlockProducerUrlMetadata {
		url: "https://cooler.stuff/spo2.json".try_into().unwrap(),
		hash: SizedByteString::from([17; 32]),
	}
}
