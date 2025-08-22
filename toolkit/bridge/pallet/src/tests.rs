use crate::mock::*;
use crate::pallet::Call;
use crate::*;
use BridgeTransferV1::*;
use core::str::FromStr;
use frame_support::{
	assert_err, assert_ok,
	inherent::{InherentData, ProvideInherent},
};
use sidechain_domain::{AssetName, MainchainAddress, PolicyId, UtxoId, bounded_str};
use sp_core::bounded_vec;
use sp_partner_chains_bridge::*;
use sp_runtime::{AccountId32, BoundedVec};

fn transfers() -> BoundedVec<BridgeTransferV1<RecipientAddress>, MaxTransfersPerBlock> {
	bounded_vec![
		UserTransfer { token_amount: 100, recipient: bounded_str!("recipient") },
		ReserveTransfer { token_amount: 200 },
		InvalidTransfer { token_amount: 300, utxo_id: UtxoId::new([1; 32], 1) }
	]
}

fn main_chain_scripts() -> MainChainScripts {
	MainChainScripts {
		token_policy_id: PolicyId([1; 28]),
		token_asset_name: AssetName(bounded_vec![2;8]),
		illiquid_supply_validator_address: MainchainAddress::from_str("validator address").unwrap(),
	}
}

fn data_checkpoint() -> BridgeDataCheckpoint {
	BridgeDataCheckpoint(UtxoId::new([1; 32], 3))
}

mod set_main_chain_scripts {
	use super::*;

	#[test]
	fn updates_scripts_in_storage() {
		new_test_ext().execute_with(|| {
			assert_ok!(Bridge::set_main_chain_scripts(RuntimeOrigin::root(), main_chain_scripts()));

			assert_eq!(Bridge::get_main_chain_scripts(), Some(main_chain_scripts()))
		})
	}

	#[test]
	fn resets_the_data_checkpoint() {
		new_test_ext().execute_with(|| {
			DataCheckpoint::<Test>::put(data_checkpoint());

			assert_ok!(Bridge::set_main_chain_scripts(RuntimeOrigin::root(), main_chain_scripts()));

			assert_eq!(Bridge::get_data_checkpoint(), None)
		})
	}
}

mod handle_transfers {
	use super::*;

	#[test]
	fn calls_the_handler() {
		new_test_ext().execute_with(|| {
			assert_ok!(Bridge::handle_transfers(
				RuntimeOrigin::none(),
				transfers(),
				data_checkpoint()
			));

			assert_eq!(mock_pallet::Transfers::<Test>::get(), Some(transfers().to_vec()));
		})
	}

	#[test]
	fn updates_the_data_checkpoint() {
		new_test_ext().execute_with(|| {
			assert_ok!(Bridge::handle_transfers(
				RuntimeOrigin::none(),
				transfers(),
				data_checkpoint()
			));

			assert_eq!(DataCheckpoint::<Test>::get(), Some(data_checkpoint()));
		})
	}

	#[test]
	fn rejects_non_extrinsic_calls() {
		new_test_ext().execute_with(|| {
			assert_err!(
				Bridge::handle_transfers(RuntimeOrigin::root(), transfers(), data_checkpoint()),
				sp_runtime::DispatchError::BadOrigin
			);

			assert_err!(
				Bridge::handle_transfers(
					RuntimeOrigin::signed(AccountId32::new(Default::default())),
					transfers(),
					data_checkpoint()
				),
				sp_runtime::DispatchError::BadOrigin
			);
		})
	}
}

mod provide_inherent {
	use super::*;

	fn inherent_data() -> InherentData {
		let mut inherent_data = InherentData::new();
		inherent_data
			.put_data(
				INHERENT_IDENTIFIER,
				&TokenBridgeTransfersV1 {
					transfers: transfers().to_vec(),
					data_checkpoint: data_checkpoint(),
				},
			)
			.expect("Putting data should succeed");
		inherent_data
	}

	#[test]
	fn creates_inherent() {
		let inherent = Bridge::create_inherent(&inherent_data()).expect("Should create inherent");

		assert_eq!(
			inherent,
			Call::handle_transfers { transfers: transfers(), data_checkpoint: data_checkpoint() }
		)
	}

	#[test]
	fn requires_inherent_when_data_present() {
		let result = Bridge::is_inherent_required(&inherent_data())
			.expect("Checking if inherent is required should not fail");

		assert_eq!(result, Some(InherentError::InherentRequired))
	}

	#[test]
	fn allows_no_inherent_when_data_missing() {
		let result = Bridge::is_inherent_required(&InherentData::new())
			.expect("Checking if inherent is required should not fail");

		assert_eq!(result, None)
	}

	#[test]
	fn verifies_inherent() {
		let correct_inherent =
			Bridge::create_inherent(&inherent_data()).expect("Should create inherent");

		assert_ok!(Bridge::check_inherent(&correct_inherent, &inherent_data()));

		let invalid_inherent = Call::handle_transfers {
			transfers: bounded_vec![],
			data_checkpoint: data_checkpoint(),
		};
		assert_err!(
			Bridge::check_inherent(&invalid_inherent, &inherent_data()),
			InherentError::IncorrectInherent
		);
	}

	#[test]
	fn only_handle_transfers_is_inherent() {
		let handle_transfers = Call::handle_transfers {
			transfers: bounded_vec![],
			data_checkpoint: data_checkpoint(),
		};

		let set_main_chain_scripts =
			Call::set_main_chain_scripts { new_scripts: main_chain_scripts() };

		assert_eq!(Bridge::is_inherent(&handle_transfers), true);
		assert_eq!(Bridge::is_inherent(&set_main_chain_scripts), false);
	}
}
