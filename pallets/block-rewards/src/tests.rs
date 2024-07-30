use super::*;
use frame_support::{
	pallet_prelude::{InherentData, ProvideInherent},
	traits::{OnFinalize, UnfilteredDispatchable},
};
use mock::*;
use sp_block_rewards::INHERENT_IDENTIFIER;

#[test]
fn on_finalize_should_create_initial_beneficiary_reward() {
	new_test_ext().execute_with(|| {
		CurrentBlockBeneficiary::<Test>::put(1);

		assert_eq!(PendingRewards::<Test>::get(1), None);
		BlockRewards::on_finalize(1);
		assert_eq!(PendingRewards::<Test>::get(1), Some(1));
	})
}

#[test]
fn on_finalize_should_increase_beneficiary_reward() {
	new_test_ext().execute_with(|| {
		CurrentBlockBeneficiary::<Test>::put(2);

		PendingRewards::<Test>::set(1, Some(2));
		PendingRewards::<Test>::set(2, Some(3));

		BlockRewards::on_finalize(1);
		assert_eq!(PendingRewards::<Test>::get(1), Some(2));
		assert_eq!(PendingRewards::<Test>::get(2), Some(4));
	})
}

#[test]
fn inherent_is_created() {
	new_test_ext().execute_with(|| {
		let mut inherent_data = InherentData::new();
		inherent_data.put_data(INHERENT_IDENTIFIER, &4).unwrap();

		let inherent = BlockRewards::create_inherent(&inherent_data);

		assert_eq!(inherent, Some(Call::set_current_block_beneficiary { beneficiary: 4 }))
	})
}

#[test]
fn inherent_sets_current_beneficiary() {
	new_test_ext().execute_with(|| {
		let inherent = Call::<Test>::set_current_block_beneficiary { beneficiary: 42 };
		assert_eq!(CurrentBlockBeneficiary::<Test>::get(), None);
		inherent
			.dispatch_bypass_filter(RuntimeOrigin::none())
			.expect("Inherent call should succeed");
		assert_eq!(CurrentBlockBeneficiary::<Test>::get(), Some(42));
	})
}
