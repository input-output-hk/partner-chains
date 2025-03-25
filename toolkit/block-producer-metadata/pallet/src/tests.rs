use super::*;
use frame_support::assert_ok;
use frame_system::pallet_prelude::OriginFor;
use hex_literal::hex;
use mock::*;
use sidechain_domain::byte_string::SizedByteString;
use sidechain_domain::*;
use sp_runtime::{AccountId32, BoundedVec};

#[test]
fn saves_new_metadata() {
	new_test_ext().execute_with(|| {
		assert_ok!(super::Pallet::<Test>::upsert_metadata(
			OriginFor::<Test>::signed(AccountId32::new([1; 32])),
			url_metadata_1(),
			cross_chain_signature_1(),
			cross_chain_pub_key(),
		));

		assert_eq!(
			Pallet::<Test>::get_metadata_for(&cross_chain_pub_key()),
			Some(url_metadata_1())
		);
	})
}

#[test]
fn updates_metadata() {
	new_test_ext().execute_with(|| {
		assert_ok!(super::Pallet::<Test>::upsert_metadata(
			OriginFor::<Test>::signed(AccountId32::new([1; 32])),
			url_metadata_1(),
			cross_chain_signature_1(),
			cross_chain_pub_key(),
		));

		assert_eq!(
			Pallet::<Test>::get_metadata_for(&cross_chain_pub_key()),
			Some(url_metadata_1())
		);

		assert_ok!(super::Pallet::<Test>::upsert_metadata(
			OriginFor::<Test>::signed(AccountId32::new([1; 32])),
			url_metadata_2(),
			cross_chain_signature_2(),
			cross_chain_pub_key(),
		));

		assert_eq!(
			Pallet::<Test>::get_metadata_for(&cross_chain_pub_key()),
			Some(url_metadata_2())
		);
	})
}

#[test]
fn rejects_invalid_signature() {
	new_test_ext().execute_with(|| {
		assert_eq!(
			super::Pallet::<Test>::upsert_metadata(
				OriginFor::<Test>::signed(AccountId32::new([1; 32])),
				url_metadata_2(),
				cross_chain_signature_1(),
				cross_chain_pub_key(),
			)
			.unwrap_err(),
			Error::<Test>::InvalidMainchainSignature.into()
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
