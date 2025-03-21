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
	use secp256k1::Secp256k1;
	new_test_ext().execute_with(|| {
		let cross_chain_pub_key: CrossChainPublicKey =
			alice_skey().public_key(&Secp256k1::new()).into();

		let message = MetadataSignedMessage {
			cross_chain_pub_key: cross_chain_pub_key.clone(),
			metadata: url_metadata_1(),
			genesis_utxo: mock::Test::genesis_utxo(),
		};

		let cross_chain_signature = message.sign_with_key(&alice_skey());

		assert_ok!(super::Pallet::<Test>::upsert_metadata(
			OriginFor::<Test>::signed(AccountId32::new([1; 32])),
			url_metadata_1(),
			cross_chain_signature,
			cross_chain_pub_key.clone(),
		));

		assert_eq!(
			Pallet::<Test>::get_partner_chain_address_for(&cross_chain_pub_key),
			Some(url_metadata_1())
		);
	})
}

#[test]
fn updates_metadata() {
	use secp256k1::Secp256k1;
	new_test_ext().execute_with(|| {
		let cross_chain_pub_key: CrossChainPublicKey =
			alice_skey().public_key(&Secp256k1::new()).into();

		let message_1 = MetadataSignedMessage {
			cross_chain_pub_key: cross_chain_pub_key.clone(),
			metadata: url_metadata_1(),
			genesis_utxo: mock::Test::genesis_utxo(),
		};

		let cross_chain_signature_1 = message_1.sign_with_key(&alice_skey());

		assert_ok!(super::Pallet::<Test>::upsert_metadata(
			OriginFor::<Test>::signed(AccountId32::new([1; 32])),
			url_metadata_1(),
			cross_chain_signature_1.clone(),
			cross_chain_pub_key.clone(),
		));

		assert_eq!(
			Pallet::<Test>::get_partner_chain_address_for(&cross_chain_pub_key),
			Some(url_metadata_1())
		);

		let message_2 = MetadataSignedMessage {
			cross_chain_pub_key: cross_chain_pub_key.clone(),
			metadata: url_metadata_2(),
			genesis_utxo: mock::Test::genesis_utxo(),
		};

		let cross_chain_signature_2 = message_2.sign_with_key(&alice_skey());

		assert_ok!(super::Pallet::<Test>::upsert_metadata(
			OriginFor::<Test>::signed(AccountId32::new([1; 32])),
			url_metadata_2(),
			cross_chain_signature_2,
			cross_chain_pub_key.clone(),
		));

		assert_eq!(
			Pallet::<Test>::get_partner_chain_address_for(&cross_chain_pub_key),
			Some(url_metadata_2())
		);
	})
}

#[test]
fn rejects_invalid_signature() {
	use secp256k1::Secp256k1;
	new_test_ext().execute_with(|| {
		let cross_chain_pub_key: CrossChainPublicKey =
			alice_skey().public_key(&Secp256k1::new()).into();

		let message = MetadataSignedMessage {
			cross_chain_pub_key: cross_chain_pub_key.clone(),
			metadata: url_metadata_1(),
			genesis_utxo: mock::Test::genesis_utxo(),
		};

		let cross_chain_signature = message.sign_with_key(&alice_skey());

		assert_eq!(
			super::Pallet::<Test>::upsert_metadata(
				OriginFor::<Test>::signed(AccountId32::new([1; 32])),
				url_metadata_2(),
				cross_chain_signature,
				cross_chain_pub_key.clone(),
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

fn url_metadata_2() -> BlockProducerUrlMetadata {
	BlockProducerUrlMetadata {
		url: BoundedVec::try_from("https://cooler.stuff/spo2.json".as_bytes().to_vec()).unwrap(),
		hash: SizedByteString::from([1; 32]),
	}
}

fn alice_skey() -> secp256k1::SecretKey {
	// Alice cross-chain key
	secp256k1::SecretKey::from_slice(&hex!(
		"cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854"
	))
	.unwrap()
}
