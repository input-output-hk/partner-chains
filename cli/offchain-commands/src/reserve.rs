use clap::Parser;
use std::error::Error;

use crate::CommonArguments;

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct ReserveCreateCmd {
	#[clap(flatten)]
	common: CommonArguments,
	total_accrued_function_script_hash: String,
	reserve_initial_incentive_amount: u128,
	reserve_posixtime_t0: u128,
	reserve_asset_script_hash: String,
	reserve_asset_name: String,
	reserve_ada_asset: String,
	reserve_initial_deposit_amount: u128,
}

impl ReserveCreateCmd {
	pub fn execute(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
		Ok(())
	}
}
