use super::*;
use frame_support::{assert_noop, assert_ok, traits::tokens::fungible::InspectHold};
use frame_system::pallet_prelude::OriginFor;
use hex_literal::hex;
use mock::*;
use sidechain_domain::byte_string::SizedByteString;
use sidechain_domain::*;
use sp_runtime::{AccountId32, BoundedVec};

#[test]
fn saves_new_metadata_and_holds_fee() {
	new_test_ext().execute_with(|| {
		let initial_balance = Balances::free_balance(&FUNDED_ACCOUNT);

		assert_ok!(super::Pallet::<Test>::upsert_metadata(
			OriginFor::<Test>::signed(FUNDED_ACCOUNT),
			url_metadata_1(),
			cross_chain_signature_1(),
			cross_chain_pub_key(),
		));

		assert_eq!(
			Pallet::<Test>::get_metadata_for(&cross_chain_pub_key()),
			Some(url_metadata_1())
		);

		let final_balance = Balances::free_balance(&FUNDED_ACCOUNT);
		assert_eq!(final_balance, initial_balance - MetadataHoldAmount::get());

		// Check that the amount is held, not burned
		let held_balance = Balances::balance_on_hold(
			&RuntimeHoldReason::BlockProducerMetadata(crate::HoldReason::MetadataDeposit),
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
		));

		assert_eq!(
			Pallet::<Test>::get_metadata_for(&cross_chain_pub_key()),
			Some(url_metadata_1())
		);
		let balance_after_insert = Balances::free_balance(&FUNDED_ACCOUNT);
		let held_after_insert = Balances::balance_on_hold(
			&RuntimeHoldReason::BlockProducerMetadata(crate::HoldReason::MetadataDeposit),
			&FUNDED_ACCOUNT,
		);

		assert_ok!(super::Pallet::<Test>::upsert_metadata(
			OriginFor::<Test>::signed(FUNDED_ACCOUNT),
			url_metadata_2(),
			cross_chain_signature_2(),
			cross_chain_pub_key(),
		));

		assert_eq!(
			Pallet::<Test>::get_metadata_for(&cross_chain_pub_key()),
			Some(url_metadata_2())
		);

		let balance_after_update = Balances::free_balance(&FUNDED_ACCOUNT);
		let held_after_update = Balances::balance_on_hold(
			&RuntimeHoldReason::BlockProducerMetadata(crate::HoldReason::MetadataDeposit),
			&FUNDED_ACCOUNT,
		);

		assert_eq!(balance_after_insert, balance_after_update);
		assert_eq!(held_after_insert, held_after_update);
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
			),
			Error::<Test>::InsufficientBalance
		);
	})
}

fn url_metadata_1() -> BlockProducerUrlMetadata {
	BlockProducerUrlMetadata {
		url: BoundedVec::try_from("https://cool.stuff/spo.json".as_bytes().to_vec()).unwrap(),
		hash: SizedByteString::from([0; 32]),
	}
}

fn cross_chain_signature_1() -> CrossChainSignature {
	CrossChainSignature(hex!(
		"e25b0291cdc8f5f7eb34e0e1586c25ee05dfb589ce9b53968bfbdeee741d2bf4430ebdd2644829ab0b7659a035fdf3d87befa05e8ec06fd22fb4092f02f6e1d6"
	).to_vec())
}

fn url_metadata_2() -> BlockProducerUrlMetadata {
	BlockProducerUrlMetadata {
		url: BoundedVec::try_from("https://cooler.stuff/spo2.json".as_bytes().to_vec()).unwrap(),
		hash: SizedByteString::from([17; 32]),
	}
}

fn cross_chain_signature_2() -> CrossChainSignature {
	CrossChainSignature(hex!(
		"6c251e9558903db7f22b93b4b6c1a3dc1088559180a70a46b12e6687ab6b3fcc7ba92798f78c7fbdf35ac6242e4862787427ff3be1c3cd55b3695cb095d13d7b"
	).to_vec())
}

fn cross_chain_pub_key() -> CrossChainPublicKey {
	// pub key of secret key cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854
	CrossChainPublicKey(
		hex!("020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1").to_vec(),
	)
}
