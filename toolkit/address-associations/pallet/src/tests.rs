use super::*;
use frame_support::assert_ok;
use frame_system::pallet_prelude::OriginFor;
use hex_literal::hex;
use mock::*;
use sidechain_domain::*;
use sp_runtime::AccountId32;

#[test]
fn saves_new_address_association() {
	new_test_ext().execute_with(|| {
		assert_eq!(mock_pallet::LastNewAssociation::<Test>::get(), None);

		assert_ok!(super::Pallet::<Test>::associate_address(
			OriginFor::<Test>::signed(FUNDED_ACCOUNT),
			pc_address(),
			VALID_SIGNATURE.into(),
			STAKE_PUBLIC_KEY.clone(),
		));

		assert_eq!(
			Pallet::<Test>::get_partner_chain_address_for(&STAKE_PUBLIC_KEY),
			Some(pc_address())
		);

		assert_eq!(
			mock_pallet::LastNewAssociation::<Test>::get(),
			Some((pc_address(), STAKE_PUBLIC_KEY.hash()))
		);
	})
}

#[test]
fn rejects_duplicate_key_association() {
	new_test_ext().execute_with(|| {
		let stake_public_key = StakePublicKey([1; 32]);
		let signature = StakeKeySignature([1; 64]);

		crate::AddressAssociations::<Test>::insert(
			&stake_public_key.hash(),
			AccountId32::new([0; 32]),
		);
		assert_eq!(
			Pallet::<Test>::get_partner_chain_address_for(&stake_public_key),
			Some(AccountId32::new([0; 32]))
		);
		assert_eq!(
			super::Pallet::<Test>::associate_address(
				OriginFor::<Test>::signed(FUNDED_ACCOUNT),
				AccountId32::new([0; 32]),
				signature,
				stake_public_key,
			)
			.unwrap_err(),
			Error::<Test>::MainchainKeyAlreadyAssociated.into()
		);
	})
}

#[test]
fn rejects_invalid_mainchain_signature() {
	new_test_ext().execute_with(|| {

			let invalid_signature = StakeKeySignature(hex!("c50828c31d1a61e05fdb943847efd42ce2eadda9c7d21dd2d035e8de66bc56de7f6b1297fba6cb7305f2aac97b5f9168894fb10295c503de6d5fb6ae70bd9a0d"));

			assert_eq!(
				super::Pallet::<Test>::associate_address(
					OriginFor::<Test>::signed(FUNDED_ACCOUNT),
					pc_address(),
					invalid_signature,
					STAKE_PUBLIC_KEY,
				)
				.unwrap_err(),
				Error::<Test>::InvalidMainchainSignature.into()
			);
		})
}

#[test]
fn rejects_extrinsic_when_origin_account_cannot_pay_extra_fee() {
	new_test_ext().execute_with(|| {
		assert_eq!(
			super::Pallet::<Test>::associate_address(
				OriginFor::<Test>::signed(AccountId32::new([3; 32])),
				pc_address(),
				VALID_SIGNATURE.into(),
				STAKE_PUBLIC_KEY,
			)
			.unwrap_err(),
			Error::<Test>::InsufficientBalance.into()
		);
	})
}
