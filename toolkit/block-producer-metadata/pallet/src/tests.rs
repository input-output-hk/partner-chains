use super::*;
use frame_support::traits::fungibles::metadata::MetadataDeposit;
use frame_support::{assert_noop, assert_ok, traits::tokens::fungible::InspectHold};
use frame_system::pallet_prelude::OriginFor;
use hex_literal::hex;
use mock::*;
use pretty_assertions::assert_eq;
use sidechain_domain::byte_string::SizedByteString;
use sidechain_domain::*;
use sp_runtime::AccountId32;

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

		let account_1_balance = Balances::free_balance(&FUNDED_ACCOUNT);
		let account_1_held = Balances::balance_on_hold(
			&RuntimeHoldReason::BlockProducerMetadata(crate::HoldReason::MetadataDeposit),
			&FUNDED_ACCOUNT,
		);

		let account_2_balance = Balances::free_balance(&FUNDED_ACCOUNT_2);
		let account_2_held = Balances::balance_on_hold(
			&RuntimeHoldReason::BlockProducerMetadata(crate::HoldReason::MetadataDeposit),
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

#[test]
fn deletes_metadata_and_returns_fee() {
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

		assert_ok!(super::Pallet::<Test>::delete_metadata(
			OriginFor::<Test>::signed(FUNDED_ACCOUNT_2),
			cross_chain_pub_key(),
			cross_chain_signature_delete(),
		));

		assert_eq!(Pallet::<Test>::get_metadata_for(&cross_chain_pub_key()), None);

		let account_1_balance = Balances::free_balance(&FUNDED_ACCOUNT);
		let account_1_held = Balances::balance_on_hold(
			&RuntimeHoldReason::BlockProducerMetadata(crate::HoldReason::MetadataDeposit),
			&FUNDED_ACCOUNT,
		);

		let account_2_balance = Balances::free_balance(&FUNDED_ACCOUNT_2);
		let account_2_held = Balances::balance_on_hold(
			&RuntimeHoldReason::BlockProducerMetadata(crate::HoldReason::MetadataDeposit),
			&FUNDED_ACCOUNT_2,
		);

		assert_eq!(account_1_balance, INITIAL_BALANCE - MetadataHoldAmount::get());
		assert_eq!(account_1_held, 0);

		assert_eq!(account_2_balance, INITIAL_BALANCE + MetadataHoldAmount::get());
		assert_eq!(account_2_held, 0);
	})
}

fn url_metadata_1() -> BlockProducerUrlMetadata {
	BlockProducerUrlMetadata {
		url: "https://cool.stuff/spo.json".try_into().unwrap(),
		hash: SizedByteString::from([0; 32]),
	}
}

fn cross_chain_signature_1() -> CrossChainSignature {
	CrossChainSignature(hex!(
		"810854f5bd1d06dc8583ebd58ff4877dddb1646511edb10afd021f716bf51a8e617353b6c5d5f92a2005e2c3c24b782a6f74132d6b54251854cce186c981862c"
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
		"0379f07264830b3e99f8fe92ff63aabec8004103253555725402a3efbd1090da232a6aeae7a083421625b544fcc4ce26964a334987982d2b398074bf16b6d481"
	).to_vec())
}

fn cross_chain_signature_delete() -> CrossChainSignature {
	CrossChainSignature(hex!(
		"5c1a701c8adffdf53a371409a24cc6c2d778a4c65c2c105c5fccfc5eeb69e3fa59bd723e7c10893f53fcfdfff8c02954f2230953cb9596119c11d4a9a29564c5"
	).to_vec())
}

fn cross_chain_pub_key() -> CrossChainPublicKey {
	// pub key of secret key cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854
	CrossChainPublicKey(
		hex!("020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1").to_vec(),
	)
}
