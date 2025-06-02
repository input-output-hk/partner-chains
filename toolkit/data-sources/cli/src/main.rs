#![deny(missing_docs)]
//! This crate provides CLI allowing usage of db-sync data sources.
//! It can be extended with more data sources.
//!
//! `follower_commands` macro is used to generate [clap] commands.
//! Command level doc comments are supported, but parameter level doc comments are not supported.

use authority_selection_inherents::authority_selection_inputs::AuthoritySelectionDataSource;
use clap::Parser;
use partner_chains_db_sync_data_sources::{BlockDataSourceImpl, CandidatesDataSourceImpl, PgPool};
use sidechain_domain::*;
use sp_timestamp::Timestamp;
use std::error::Error;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

#[tokio::main]
async fn main() {
	env_logger::builder().filter_level(log::LevelFilter::Info).init();
	match Command::parse().run().await {
		Ok(resp) => println!("{resp}"),
		Err(err) => log::error!("{}", err.to_string()),
	}
}

macro_rules! follower_commands {
	(
		$(
			$ds:ident {
				$(
					$(#[$cmd_comment:meta])*
					async fn $method:ident($($i:literal $arg:ident : $arg_ty:ty),*);
				)+
			}
		)*
	) => {
		#[derive(Debug, clap::Parser)]
		#[allow(non_camel_case_types)]
		enum Command {
			$(
				$(
					$(#[$cmd_comment])*
					$method {
						$(
							#[clap(index = $i)]
							$arg: $arg_ty
						),*
					}
				),*
			),*
		}
		impl Command {
			async fn run(self) -> Result<String> {
				match self {
					$(
						$(
							Command::$method { $($arg),* } => {
								let data_source = crate::data_source::$ds().await.expect("Failed to create data source");
								let result = data_source.$method($($arg),*).await?;
								let result = serde_json::to_string_pretty(&result)?;
								Ok(result)
							}
						),*
					),*
				}
			}
		}
    };
}

follower_commands! {
	block {
		/// Returns the latest block recorded by DB-Sync
		async fn get_latest_block_info();
		/// Returns data of block that was the latest stable block at given timestamp
		async fn get_latest_stable_block_for(1 reference_timestamp_millis: u64);
		/// Returns data of the block identified by given hash, but only if the block can be considered stable in relation to reference timestamp
		async fn get_stable_block_for(1 hash: McBlockHash, 2 reference_timestamp_millis: u64);
	}
	candidate {
		/// Returns values of D-parameter and Permissioned Candidates effective at given epoch. Policy IDs should be hex encoded.
		async fn get_ariadne_parameters(1 epoch_number: McEpochNumber, 2 d_parameter_policy_id: PolicyId, 3 permissioned_candidates_policy_id: PolicyId);
		/// Returns registered candidates data effective at given Cardano epoch.
		async fn get_candidates(1 epoch_number: McEpochNumber, 2 committee_candidate_validator_address: MainchainAddress);
		/// Returns Cardano epoch nonce used by committee selection during given Cardano epoch. It is not nonce of the given epoch.
		async fn get_epoch_nonce(1 epoch_number: McEpochNumber);
	}
}

mod data_source {

	use super::*;

	async fn pool() -> Result<PgPool> {
		partner_chains_db_sync_data_sources::get_connection_from_env().await
	}

	pub(crate) struct BlockDataSourceWrapper {
		inner: BlockDataSourceImpl,
	}

	impl BlockDataSourceWrapper {
		pub(crate) async fn get_latest_block_info(&self) -> Result<MainchainBlock> {
			self.inner.get_latest_block_info().await
		}

		pub(crate) async fn get_latest_stable_block_for(
			&self,
			reference_timestamp: u64,
		) -> Result<Option<MainchainBlock>> {
			self.inner
				.get_latest_stable_block_for(Timestamp::new(reference_timestamp))
				.await
		}

		pub(crate) async fn get_stable_block_for(
			&self,
			hash: McBlockHash,
			reference_timestamp: u64,
		) -> Result<Option<MainchainBlock>> {
			self.inner.get_stable_block_for(hash, Timestamp::new(reference_timestamp)).await
		}
	}

	pub(crate) async fn block() -> Result<BlockDataSourceWrapper> {
		let cardano_config = sidechain_domain::cardano_config::CardanoConfig::from_env()?;
		Ok(BlockDataSourceWrapper {
			inner: BlockDataSourceImpl::new_from_env(pool().await?, &cardano_config).await?,
		})
	}

	pub(crate) async fn candidate() -> Result<CandidatesDataSourceImpl> {
		CandidatesDataSourceImpl::new(pool().await?, None).await
	}
}
