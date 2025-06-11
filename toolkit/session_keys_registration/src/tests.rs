use frame_support::assert_ok;
use sp_runtime::testing::UintAuthorityId;

use crate::mock::{
	OriginCaller, Session, SessionKeys, SessionKeysRegistration, System, new_test_ext,
	start_session,
};

#[test]
fn register_session_keys_for_provided_authorities() {
	new_test_ext().execute_with(|| {
		start_session(1);

		{
			let account_id: u64 = 1;
			System::inc_providers(&account_id);

			// By default, the session keys are not set for the account.
			assert_eq!(Session::load_keys(&account_id), None);

			let as_origin =
				Box::new(OriginCaller::system(frame_system::RawOrigin::Signed(account_id)));
			let keys = SessionKeys { foo: UintAuthorityId::from(42) };
			let proof = Default::default();

			assert_ok!(SessionKeysRegistration::set_keys(as_origin, keys, proof));

			// After setting the keys, they should be stored in the session.
			assert_eq!(
				Session::load_keys(&account_id),
				Some(SessionKeys { foo: UintAuthorityId(42) })
			);
		}
	});
}
