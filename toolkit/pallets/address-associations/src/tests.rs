use super::*;
use frame_support::assert_ok;
use frame_system::pallet_prelude::OriginFor;
use hex_literal::hex;
use mock::*;
use sidechain_domain::*;
use sp_runtime::AccountId32;

use sp_core::crypto::Ss58Codec;

use super::*;

#[test]
fn saves_new_address_association() {
	new_test_ext().execute_with(|| {
			// Alice
			let mc_pub_key = MainchainPublicKey(hex!(
				"2bebcb7fbc74a6e0fd6e00a311698b047b7b659f0e047ff5349dbd984aefc52c"
			));
			let mc_signature = hex!("1aa8c1b363a207ddadf0c6242a0632f5a557690a327d0245f9d473b983b3d8e1c95a3dd804cab41123c36ddbcb7137b8261c35d5c8ef04ce9d0f8d5c4b3ca607");

			// Alice
			let pc_address =
					AccountId32::from_ss58check("5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY").unwrap();

			assert_ok!(
				super::Pallet::<Test>::associate_address(
					OriginFor::<Test>::signed(AccountId32::new([1; 32])),
					pc_address.clone(),
					mc_signature,
					mc_pub_key.clone(),
				)
			);

			assert_eq!(
				Pallet::<Test>::get_partner_chain_address_for(&mc_pub_key),
				Some(pc_address)
			);
		})
}

#[test]
fn rejects_duplicate_key_association() {
	new_test_ext().execute_with(|| {
		let mc_pub_key = MainchainPublicKey([1; 32]);
		let mc_signarture = [1; 64];

		crate::AddressAssociations::<Test>::insert(&mc_pub_key.hash(), AccountId32::new([0; 32]));
		assert_eq!(
			Pallet::<Test>::get_partner_chain_address_for(&mc_pub_key),
			Some(AccountId32::new([0; 32]))
		);
		assert_eq!(
			super::Pallet::<Test>::associate_address(
				OriginFor::<Test>::signed(AccountId32::new([1; 32])),
				AccountId32::new([0; 32]),
				mc_signarture,
				mc_pub_key,
			)
			.unwrap_err(),
			Error::<Test>::MainchainKeyAlreadyAssociated.into()
		);
	})
}

#[test]
fn rejects_invalid_mainchain_signature() {
	new_test_ext().execute_with(|| {
			let mc_pub_key = MainchainPublicKey(hex!(
				"fc014cb5f071f5d6a36cb5a7e5f168c86555989445a23d4abec33d280f71aca4"
			));
			let mc_signarture = hex!("c50828c31d1a61e05fdb943847efd42ce2eadda9c7d21dd2d035e8de66bc56de7f6b1297fba6cb7305f2aac97b5f9168894fb10295c503de6d5fb6ae70bd9a0d");

			assert_eq!(
				super::Pallet::<Test>::associate_address(
					OriginFor::<Test>::signed(AccountId32::new([1; 32])),
					AccountId32::new([0; 32]),
					mc_signarture,
					mc_pub_key,
				)
				.unwrap_err(),
				Error::<Test>::InvalidMainchainSignature.into()
			);
		})
}
