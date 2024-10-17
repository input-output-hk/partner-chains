use authority_selection_inherents::authority_selection_inputs::AuthoritySelectionDataSource;
use clap::Parser;
use db_sync_follower::{
	block::{BlockDataSourceImpl, DbSyncBlockDataSourceConfig},
	candidates::CandidatesDataSourceImpl,
	data_sources::{read_mc_epoch_config, PgPool},
};
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
		async fn get_latest_stable_block_for(1 reference_timestamp: u64);
		async fn get_stable_block_for(1 hash: McBlockHash, 2 reference_timestamp: u64);
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

	pub struct BlockDataSourceWrapper {
		inner: BlockDataSourceImpl,
	}

	impl BlockDataSourceWrapper {
		pub async fn get_latest_block_info(&self) -> Result<MainchainBlock> {
			self.inner.get_latest_block_info().await
		}

		pub async fn get_latest_stable_block_for(
			&self,
			reference_timestamp: u64,
		) -> Result<Option<MainchainBlock>> {
			self.inner
				.get_latest_stable_block_for(Timestamp::new(reference_timestamp))
				.await
		}

		pub async fn get_stable_block_for(
			&self,
			hash: McBlockHash,
			reference_timestamp: u64,
		) -> Result<Option<MainchainBlock>> {
			self.inner.get_stable_block_for(hash, Timestamp::new(reference_timestamp)).await
		}
	}

	pub async fn block() -> Result<BlockDataSourceWrapper> {
		Ok(BlockDataSourceWrapper {
			inner: BlockDataSourceImpl::from_config(
				pool().await?,
				DbSyncBlockDataSourceConfig::from_env()?,
				&read_mc_epoch_config()?,
			),
		})
	}

	pub async fn candidate() -> Result<CandidatesDataSourceImpl> {
		Ok(CandidatesDataSourceImpl::new(pool().await?, None).await?)
	}
}
