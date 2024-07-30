use chain_params::SidechainParams;
use clap::Parser;
use sc_cli::{CliConfiguration, SharedParams};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use sp_sidechain::GetSidechainParams;
use std::io::Write;
use std::sync::Arc;

#[derive(Debug, Clone, Parser)]
pub struct SidechainParamsCmd {
	#[allow(missing_docs)]
	#[clap(flatten)]
	pub shared_params: SharedParams,
}
impl SidechainParamsCmd {
	pub async fn run<B, C>(&self, client: Arc<C>) -> sc_cli::Result<()>
	where
		B: BlockT,
		C: ProvideRuntimeApi<B> + Send + Sync + 'static,
		C::Api: GetSidechainParams<B, SidechainParams>,
		C: HeaderBackend<B>,
	{
		let api = client.runtime_api();
		let best_block = client.info().best_hash;
		let sidechain_params = api.sidechain_params(best_block).unwrap();
		let output = serde_json::to_string_pretty(&sidechain_params).unwrap();
		std::io::stdout().write_all(output.as_bytes()).unwrap();
		Ok(())
	}
}

impl CliConfiguration for SidechainParamsCmd {
	fn shared_params(&self) -> &SharedParams {
		&self.shared_params
	}
}
