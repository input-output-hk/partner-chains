use crate::mock::*;
use crate::*;
use authority_selection_inherents::AriadneInherentDataProvider;
use frame_support::{
	dispatch::PostDispatchInfo,
	inherent::ProvideInherent,
	traits::{UnfilteredDispatchable, WhitelistedStorageKeys},
};
use pretty_assertions::assert_eq;
use sp_core::{Pair, hexdisplay::HexDisplay};
use sp_inherents::InherentData;
use std::collections::HashSet;

#[test]
fn check_whitelist() {
	let whitelist: HashSet<String> = super::AllPalletsWithSystem::whitelisted_storage_keys()
		.iter()
		.map(|e| HexDisplay::from(&e.key).to_string())
		.collect();

	// Block Number
	assert!(whitelist.contains("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac"));
	// Total Issuance
	assert!(whitelist.contains("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80"));
	// Execution Phase
	assert!(whitelist.contains("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a"));
	// Event Count
	assert!(whitelist.contains("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850"));
	// System Events
	assert!(whitelist.contains("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7"));
}

// The set committee takes effect next session. Committee can be set for 1 session in advance.
#[test]
fn check_grandpa_authorities_rotation() {
	new_test_ext().execute_with(|| {
		// Needs to be run to initialize first slot and epoch numbers;
		advance_block();

		// Committee goes into effect 1-epoch and 1-block after selection
		set_committee_through_inherent_data(&[alice()]);
		until_epoch_after_finalizing(1, &|| {
			assert_grandpa_weights();
			assert_grandpa_authorities!([alice(), bob()]);
		});
		for_next_n_blocks_after_finalizing(1, &|| {
			assert_grandpa_weights();
			assert_grandpa_authorities!([alice(), bob()]);
		});
		set_committee_through_inherent_data(&[bob()]);
		for_next_n_blocks_after_finalizing(SLOTS_PER_EPOCH, &|| {
			assert_grandpa_weights();
			assert_grandpa_authorities!([alice()]);
		});
		set_committee_through_inherent_data(&[alice()]);
		for_next_n_blocks_after_finalizing(SLOTS_PER_EPOCH, &|| {
			assert_grandpa_weights();
			assert_grandpa_authorities!([bob()]);
		});
		set_committee_through_inherent_data(&[alice(), bob()]);
		for_next_n_blocks_after_finalizing(SLOTS_PER_EPOCH, &|| {
			assert_grandpa_weights();
			assert_grandpa_authorities!([alice()]);
		});
		set_committee_through_inherent_data(&[bob(), alice()]);
		for_next_n_blocks_after_finalizing(SLOTS_PER_EPOCH, &|| {
			assert_grandpa_weights();
			assert_grandpa_authorities!([alice(), bob()]);
		});
		set_committee_through_inherent_data(&[alice()]);
		for_next_n_blocks_after_finalizing(SLOTS_PER_EPOCH, &|| {
			assert_grandpa_weights();
			assert_grandpa_authorities!([bob(), alice()]);
		});

		// When there's no new committees being scheduled, the last committee stays in power
		for_next_n_blocks_after_finalizing(SLOTS_PER_EPOCH * 3, &|| {
			assert_grandpa_weights();
			assert_grandpa_authorities!([alice()]);
		});
	});

	fn assert_grandpa_weights() {
		Grandpa::grandpa_authorities()
			.into_iter()
			.for_each(|(_, weight)| assert_eq!(weight, 1))
	}
}

// The set committee takes effect next session. Committee can be set for 1 session in advance.
#[test]
fn check_aura_authorities_rotation() {
	new_test_ext().execute_with(|| {
		// Needs to be run to initialize first slot and epoch numbers;
		advance_block();
		// Committee goes into effect 1-epoch and 1-block after selection
		set_committee_through_inherent_data(&[alice()]);
		until_epoch_after_finalizing(1, &|| {
			assert_aura_authorities!([alice(), bob()]);
		});
		for_next_n_blocks_after_finalizing(1, &|| {
			assert_aura_authorities!([alice(), bob()]);
		});
		set_committee_through_inherent_data(&[bob()]);
		for_next_n_blocks_after_finalizing(SLOTS_PER_EPOCH, &|| {
			assert_aura_authorities!([alice()]);
		});
		set_committee_through_inherent_data(&[alice()]);
		for_next_n_blocks_after_finalizing(SLOTS_PER_EPOCH, &|| {
			assert_aura_authorities!([bob()]);
		});
		set_committee_through_inherent_data(&[alice(), bob()]);
		for_next_n_blocks_after_finalizing(SLOTS_PER_EPOCH, &|| {
			assert_aura_authorities!([alice()]);
		});
		set_committee_through_inherent_data(&[bob(), alice()]);
		for_next_n_blocks_after_finalizing(SLOTS_PER_EPOCH, &|| {
			assert_aura_authorities!([alice(), bob()]);
		});
		for_next_n_blocks_after_finalizing(SLOTS_PER_EPOCH, &|| {
			assert_aura_authorities!([alice(), bob()]);
		});

		// When there's no new committees being scheduled, the last committee stays in power
		for_next_n_blocks_after_finalizing(SLOTS_PER_EPOCH * 3, &|| {
			assert_aura_authorities!([alice(), bob()]);
		});
	});
}

// The set committee takes effect at next session. Committee can be set for 1 session in advance.
#[test]
fn check_cross_chain_committee_rotation() {
	new_test_ext().execute_with(|| {
		advance_block();
		set_committee_through_inherent_data(&[alice()]);
		until_epoch(1, &|| {
			assert_current_epoch!(0);
			assert_next_committee!([alice()]);
		});

		set_committee_through_inherent_data(&[bob()]);
		for_next_n_blocks(SLOTS_PER_EPOCH, &|| {
			assert_current_epoch!(1);
			assert_next_committee!([bob()]);
		});

		set_committee_through_inherent_data(&[]);
		for_next_n_blocks(SLOTS_PER_EPOCH, &|| {
			assert_current_epoch!(2);
			assert_next_committee!([bob()]);
		});
	});
}

fn set_committee_through_inherent_data(expected_authorities: &[TestKeys]) -> PostDispatchInfo {
	let epoch = Sidechain::current_epoch_number();
	let slot = *pallet_aura::CurrentSlot::<Runtime>::get();
	println!(
		"(slot {slot}, epoch {epoch}) Setting {} authorities for next epoch",
		expected_authorities.len()
	);
	let inherent_data_struct = create_inherent_data_struct(expected_authorities);
	let ariadne_selection_inputs = match inherent_data_struct {
		AriadneInherentDataProvider::Inert => panic!("Inert inherent data provider"),
		AriadneInherentDataProvider::Legacy(inputs) => inputs.unwrap().into(),
		AriadneInherentDataProvider::V1(inputs) => inputs.unwrap(),
	};

	let mut inherent_data = InherentData::new();
	inherent_data
		.put_data(SessionCommitteeManagement::INHERENT_IDENTIFIER, &ariadne_selection_inputs)
		.expect("Setting inherent data should not fail");
	let call = <SessionCommitteeManagement as ProvideInherent>::create_inherent(&inherent_data)
		.expect("Creating test inherent should not fail");
	println!("    inherent: {:?}", call);
	call.dispatch_bypass_filter(RuntimeOrigin::none())
		.expect("dispatching test call should work")
}
