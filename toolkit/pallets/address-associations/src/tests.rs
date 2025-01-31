use super::*;
use frame_support::assert_ok;
use frame_system::pallet_prelude::OriginFor;
use hex_literal::hex;
use mock::*;
use sidechain_domain::*;
use sp_runtime::AccountId32;
use std::str::FromStr;

mod inherent {
	use super::*;

	#[test]
	fn saves_new_address_association() {
		new_test_ext().execute_with(|| {
			let mc_address = MainchainAddress::from_str(
				"addr_test1vr5vxqpnpl3325cu4zw55tnapjqzzx78pdrnk8k5j7wl72c6y08nd",
			)
			.unwrap();
			let mc_pub_key = MainchainPublicKey(hex!(
				"fc014cb5f071f5d6a36cb5a7e5f168c86555989445a23d4abec33d280f71aca4"
			));
			let mc_signarture = MainchainSignature(hex!("b50828c31d1a61e05fdb943847efd42ce2eadda9c7d21dd2d035e8de66bc56de7f6b1297fba6cb7305f2aac97b5f9168894fb10295c503de6d5fb6ae70bd9a0d"));

			assert_ok!(
				super::Pallet::<Test>::associate_address(
					OriginFor::<Test>::signed(AccountId32::new([1; 32])),
					mc_address.clone(),
					AccountId32::new([0; 32]),
					mc_signarture,
					mc_pub_key,
				)
			);

			assert_eq!(
				Pallet::<Test>::get_partner_chain_address_for(&mc_address),
				Some(AccountId32::new([0; 32]))
			);
		})
	}

	#[test]
	fn rejects_duplicate_key_association() {
		new_test_ext().execute_with(|| {
			let mc_address = MainchainAddress::from_str(
				"addr_test1wpvtw0r3acskmgrphlz0m4c57djpxdvw9gdcwx0rwter9acadwzzc",
			)
			.unwrap();
			let mc_pub_key = MainchainPublicKey([1; 32]);
			let mc_signarture = MainchainSignature([1; 64]);

			crate::AddressAssociations::<Test>::insert(&mc_address, AccountId32::new([0; 32]));
			assert_eq!(
				Pallet::<Test>::get_partner_chain_address_for(&mc_address),
				Some(AccountId32::new([0; 32]))
			);
			assert_eq!(
				super::Pallet::<Test>::associate_address(
					OriginFor::<Test>::signed(AccountId32::new([1; 32])),
					mc_address,
					AccountId32::new([0; 32]),
					mc_signarture,
					mc_pub_key,
				)
				.unwrap_err(),
				Error::<Test>::AddressAlreadyAssociated.into()
			);
		})
	}

	#[test]
	fn rejects_invalid_mainchain_address() {
		new_test_ext().execute_with(|| {
			let mc_address = MainchainAddress::from_str(
				"addr_test2wpvtw0r3acskmgrphlz0m4c57djpxdvw9gdcwx0rwter9acadwzzc",
			)
			.unwrap();
			let mc_pub_key = MainchainPublicKey([1; 32]);
			let mc_signarture = MainchainSignature([1; 64]);

			assert_eq!(
				super::Pallet::<Test>::associate_address(
					OriginFor::<Test>::signed(AccountId32::new([1; 32])),
					mc_address,
					AccountId32::new([0; 32]),
					mc_signarture,
					mc_pub_key,
				)
				.unwrap_err(),
				Error::<Test>::InvalidMainchainAddress.into()
			);
		})
	}

	#[test]
	fn rejects_invalid_mainchain_public_key() {
		new_test_ext().execute_with(|| {
			let mc_address = MainchainAddress::from_str(
				"addr_test1wpvtw0r3acskmgrphlz0m4c57djpxdvw9gdcwx0rwter9acadwzzc",
			)
			.unwrap();
			let mc_pub_key = MainchainPublicKey([2; 32]);
			let mc_signarture = MainchainSignature([1; 64]);

			assert_eq!(
				super::Pallet::<Test>::associate_address(
					OriginFor::<Test>::signed(AccountId32::new([1; 32])),
					mc_address,
					AccountId32::new([0; 32]),
					mc_signarture,
					mc_pub_key,
				)
				.unwrap_err(),
				Error::<Test>::InvalidMainchainPublicKey.into()
			);
		})
	}

	#[test]
	fn rejects_invalid_mainchain_signature() {
		new_test_ext().execute_with(|| {
			let mc_address = MainchainAddress::from_str(
				"addr_test1vr5vxqpnpl3325cu4zw55tnapjqzzx78pdrnk8k5j7wl72c6y08nd",
			)
				.unwrap();
			let mc_pub_key = MainchainPublicKey(hex!(
				"fc014cb5f071f5d6a36cb5a7e5f168c86555989445a23d4abec33d280f71aca4"
			));
			let mc_signarture = MainchainSignature(hex!("c50828c31d1a61e05fdb943847efd42ce2eadda9c7d21dd2d035e8de66bc56de7f6b1297fba6cb7305f2aac97b5f9168894fb10295c503de6d5fb6ae70bd9a0d"));

			assert_eq!(
				super::Pallet::<Test>::associate_address(
					OriginFor::<Test>::signed(AccountId32::new([1; 32])),
					mc_address,
					AccountId32::new([0; 32]),
					mc_signarture,
					mc_pub_key,
				)
				.unwrap_err(),
				Error::<Test>::InvalidMainchainSignature.into()
			);
		})
	}
}
