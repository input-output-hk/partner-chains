use crate::HoldReason;

use super::*;
use frame_support::{assert_noop, assert_ok, traits::tokens::fungible::InspectHold};
use frame_system::pallet_prelude::OriginFor;
use hex_literal::hex;
use mock::*;
use sidechain_domain::byte_string::SizedByteString;
use sidechain_domain::*;
use sp_runtime::AccountId32;

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
				cross_chain_signature_1(),
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
				cross_chain_signature_1(),
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
				cross_chain_signature_2(),
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
					cross_chain_signature_1(),
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
			let poor_account = AccountId32::new([13; 32]);

			assert_noop!(
				super::Pallet::<Test>::upsert_metadata(
					OriginFor::<Test>::signed(poor_account),
					url_metadata_1(),
					cross_chain_signature_1(),
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
				cross_chain_signature_1(),
				cross_chain_pub_key(),
				valid_before(),
			)
			.expect("First insert should succeed");

			let error = super::Pallet::<Test>::upsert_metadata(
				OriginFor::<Test>::signed(FUNDED_ACCOUNT_2),
				url_metadata_1(),
				cross_chain_signature_1(),
				cross_chain_pub_key(),
				valid_before(),
			)
			.unwrap_err();
			assert_eq!(error, Error::<Test>::NotTheOwner.into());
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
				cross_chain_signature_1(),
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
				cross_chain_signature_delete(),
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
				cross_chain_signature_1(),
				cross_chain_pub_key(),
				valid_before(),
			)
			.expect("First insert should succeed");

			let error = super::Pallet::<Test>::delete_metadata(
				OriginFor::<Test>::signed(FUNDED_ACCOUNT_2),
				cross_chain_pub_key(),
				cross_chain_signature_delete(),
				valid_before(),
			)
			.unwrap_err();
			assert_eq!(error, Error::<Test>::NotTheOwner.into());
		})
	}
}

fn url_metadata_1() -> BlockProducerUrlMetadata {
	BlockProducerUrlMetadata {
		url: "https://cool.stuff/spo.json".try_into().unwrap(),
		hash: SizedByteString::from([0; 32]),
	}
}

fn cross_chain_signature_1() -> CrossChainSignature {
	CrossChainSignature(hex!(
		"0e644ae5589365cce0123e673d59eab5381a1c38d5e21a7732bce8592f38fd522e9d395584f72b03ad9b167c1f57813013e0c6feedea799f877f87ec4edc3177"
	).to_vec())
}

fn url_metadata_2() -> BlockProducerUrlMetadata {
	BlockProducerUrlMetadata {
		url: "https://cooler.stuff/spo2.json".try_into().unwrap(),
		hash: SizedByteString::from([17; 32]),
	}
}

fn cross_chain_signature_2() -> CrossChainSignature {
	CrossChainSignature(hex!(
		"1dc03e8577bfda40215ce2e392f2bccb7d203664fa8b031ba14b27dbf2e7e2af345bcb5424e5b2e31ec2027d8313c25a6cbc21ebcdeadee398aaaa6491fb3a02"
	).to_vec())
}

fn cross_chain_signature_delete() -> CrossChainSignature {
	CrossChainSignature(hex!(
		"28e26efe063733903d79bcd2a036b2f2050e6d54372ad0dbf9db2bcd2026ce58171826fcd205c74c5cdd4cda08a3d5e1497b3d968f3d9328e816b3a9166a68d9"
	).to_vec())
}

fn cross_chain_pub_key() -> CrossChainPublicKey {
	// pub key of secret key cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854
	CrossChainPublicKey(
		hex!("020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1").to_vec(),
	)
}

fn valid_before() -> u64 {
	100_000_000
}
