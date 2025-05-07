//! Implements storage migrations of `session-validator-management` pallet.
//!
//! **Important:** It is crucial to run the migrations when upgrading runtime
//! to a version containing this pallet's storage version. Failing to do so
//! WILL break the chain and require a wipe or a rollback.
//!
//! To schedule a migration, add it to the `Executive` definition for your
//! runtime like this:
//!
//! ```rust, ignore
//! pub type Migrations = (
//! 	pallet_session_validator_management::migrations::v1::LegacyToV1Migration<Runtime>,
//!     // ...
//! );
//! /// Executive: handles dispatch to the various modules.
//! pub type Executive = frame_executive::Executive<
//! 	Runtime,
//! 	Block,
//! 	ChainContext,
//! 	Runtime,
//! 	AllPalletsWithSystem,
//! 	Migrations,
//! >;
//! ```
//!
//! Each migration will be run only once, for the storage version for which it is
//! defined, and will update the storage version number.
//!
//! ## V1 (v1.6.0+)
//!
//! ### Changes
//!
//! This version changes the type used to store committee member information
//! from a tuple `(T::AuthorityId, T::AuthorityKeys)` to a generic type
//! `T::CommitteeMember`. This type can be arbitrary within normal
//! constraints of Substrate runtime types and must implement the
//! `CommitteeMember` trait. If your runtime uses `authority-selection-inherents`
//! to select its committee, use the `CommitteeMember` type provided by this crate.
//! A `CommitteeMember` implementation for the legacy `(T::AuthorityId, T::AuthorityKeys)`
//! type is also provided and can be used.
//!
//! ### Migration from Legacy
//!
//! Migration logic is provided by the `migrations::v1::LegacyToV1Migration` migration.
//! It assumes that the types `T::AuthorityId` and `T::AuthorityKeys` do not change as
//! part of the same runtime upgrade. The only requirement for the new type
//! `T::CommitteeMember` is to implement the trait `From<(T::AuthorityId, T::AuthorityKeys)>`.

pub mod v0;
pub mod v1;
