use std::error::Error;

use clap::Parser;
use db_sync_follower::{
	block::{BlockDataSourceImpl, DbSyncBlockDataSourceConfig},
	candidates::CandidatesDataSourceImpl,
	data_sources::{read_mc_epoch_config, PgPool},
};
use main_chain_follower_api::{common::*, *};
use sidechain_domain::*;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

#[tokio::main]
async fn main() {
	env_logger::builder().filter_level(log::LevelFilter::Info).init();
	match Command::parse().run().await {
		Ok(resp) => println!("{resp}"),
		Err(err) => log::error!("{}", err.to_string()),
	}
}

pub struct DataSources {
	pub block: BlockDataSourceImpl,
	pub candidate: CandidatesDataSourceImpl,
}

macro_rules! follower_commands {
	(
		$(
			$ds:ident {
				$(
					async fn $method:ident($($i:literal $arg:ident : $arg_ty:ty),*);
				)+
			}
		)*
	) => {
		#[derive(Debug, clap::Parser)]
		#[allow(non_camel_case_types)]
		pub enum Command {
			$(
				$(
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
			pub async fn run(self) -> Result<String> {
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
		async fn get_latest_block_info();
		async fn get_latest_stable_block_for(1 reference_timestamp: Timestamp);
		async fn get_stable_block_for(1 hash: McBlockHash, 2 reference_timestamp: Timestamp);
	}
	candidate {
		async fn get_ariadne_parameters(1 epoch_number: McEpochNumber, 2 d_parameter_policy: PolicyId, 3 permissioned_candidates_policy: PolicyId);
		async fn get_candidates(1 epoch_number: McEpochNumber, 2 committee_candidate_validator: MainchainAddress);
		async fn get_epoch_nonce(1 epoch_number: McEpochNumber);
	}
}

mod data_source {

	use super::*;

	async fn pool() -> Result<PgPool> {
		db_sync_follower::data_sources::get_connection_from_env().await
	}

	pub async fn block() -> Result<BlockDataSourceImpl> {
		Ok(BlockDataSourceImpl::from_config(
			pool().await?,
			DbSyncBlockDataSourceConfig::from_env()?,
			&read_mc_epoch_config()?,
			None,
		))
	}

	pub async fn candidate() -> Result<CandidatesDataSourceImpl> {
		Ok(CandidatesDataSourceImpl::from_config(pool().await?, None).await?)
	}
}
