use async_trait::async_trait;
use blockfrost_openapi::models::{
	address_transactions_content_inner::AddressTransactionsContentInner,
	address_utxo_content_inner::AddressUtxoContentInner,
	asset_addresses_inner::AssetAddressesInner, asset_transactions_inner::AssetTransactionsInner,
	block_content::BlockContent, epoch_param_content::EpochParamContent,
	epoch_stake_pool_content_inner::EpochStakePoolContentInner, genesis_content::GenesisContent,
	pool_history_inner::PoolHistoryInner, pool_list_extended_inner::PoolListExtendedInner,
	tx_content::TxContent, tx_content_utxo::TxContentUtxo,
};
use serde::de::DeserializeOwned;
use sidechain_domain::*;
use std::time::Duration;
use ureq::Agent;

use crate::{
	DataSourceError,
	client::api::{McBlockId, McPoolId, MiniBFApi},
};

/// Client implementing Dolos MiniBF
#[derive(Clone)]
pub struct MiniBFClient {
	agent: ureq::Agent,
	addr: String,
}

impl MiniBFClient {
	pub fn new(addr: &str, timeout: Duration) -> Self {
		let agent = Agent::config_builder().timeout_per_call(Some(timeout)).build().into();
		MiniBFClient { agent, addr: addr.strip_suffix("/").unwrap_or(addr).to_string() }
	}

	async fn request<T: DeserializeOwned + std::fmt::Debug>(
		&self,
		method: &str,
	) -> Result<T, DataSourceError> {
		let req = format!("{}/{}", self.addr, method);
		log::trace!("Dolos request: {req:?}");
		let resp = self
			.agent
			.get(req)
			.call()
			.map_err(|e| DataSourceError::DolosCallError(e.to_string()))
			.and_then(|mut r| {
				r.body_mut()
					.read_json()
					.map_err(|e| DataSourceError::DolosResponseParseError(e.to_string()))
			});
		log::trace!("Dolos response: {resp:?}");
		resp
	}

	async fn paginated_request<T: DeserializeOwned + std::fmt::Debug>(
		&self,
		method: &str,
		pagination: Pagination,
	) -> Result<Vec<T>, DataSourceError> {
		let mut query_pairs = url::form_urlencoded::Serializer::new(String::new());
		query_pairs.extend_pairs([
			("count", &pagination.count.to_string()),
			("page", &pagination.page.to_string()),
			("order", &pagination.order.to_string()),
		]);
		if let Some(from) = pagination.from {
			query_pairs.append_pair("from", &from);
		}
		if let Some(to) = pagination.to {
			query_pairs.append_pair("to", &to);
		}
		let mut req_url =
			url::Url::parse(&format!("{}/{}", self.addr, method)).expect("valid Dolos url");
		req_url.set_query(Some(&query_pairs.finish()));
		log::trace!("Dolos request: {req_url:?}");
		let resp = match self.agent.get(req_url.as_str()).call() {
			Ok(mut r) => r
				.body_mut()
				.read_json()
				.map_err(|e| DataSourceError::DolosResponseParseError(e.to_string())),
			Err(ureq::Error::StatusCode(404)) => {
				// Handle 404 as empty result for paginated requests (e.g., no UTXOs at address)
				log::debug!("Dolos returned 404 for {req_url:?}, treating as empty result");
				Ok(Vec::new())
			}
			Err(e) => Err(DataSourceError::DolosCallError(e.to_string())),
		};
		log::trace!("Dolos response: {resp:?}");
		resp
	}

	async fn paginated_request_all<T: DeserializeOwned + std::fmt::Debug>(
		&self,
		method: &str,
	) -> Result<Vec<T>, DataSourceError> {
		let mut pagination: Pagination = Pagination::default();
		let mut have_all_pages = false;
		let mut res = Vec::new();
		while !have_all_pages {
			let mut resp: Vec<T> = self.paginated_request(method, pagination.clone()).await?;
			if (resp.len() as i32) < pagination.count {
				have_all_pages = true
			}
			res.append(&mut resp);
			pagination.page += 1;
		}
		Ok(res)
	}
}

#[async_trait]
impl MiniBFApi for MiniBFClient {
	async fn addresses_utxos(
		&self,
		address: MainchainAddress,
	) -> Result<Vec<AddressUtxoContentInner>, DataSourceError> {
		self.paginated_request_all(&format!("addresses/{address}/utxos")).await
	}

	async fn addresses_utxos_asset(
		&self,
		address: MainchainAddress,
		asset: AssetId,
	) -> Result<Vec<AddressUtxoContentInner>, DataSourceError> {
		let asset_id_str = format_asset_id(&asset);
		self.paginated_request_all(&format!("addresses/{address}/utxos/{asset_id_str}"))
			.await
	}

	async fn addresses_transactions(
		&self,
		address: MainchainAddress,
	) -> Result<Vec<AddressTransactionsContentInner>, DataSourceError> {
		self.paginated_request_all(&format!("addresses/{address}/transactions")).await
	}

	async fn assets_transactions(
		&self,
		asset_id: AssetId,
	) -> Result<Vec<AssetTransactionsInner>, DataSourceError> {
		let asset_id_str = format_asset_id(&asset_id);
		self.paginated_request_all(&format!("assets/{asset_id_str}/transactions")).await
	}

	async fn assets_addresses(
		&self,
		asset_id: AssetId,
	) -> Result<Vec<AssetAddressesInner>, DataSourceError> {
		let asset_id_str = format_asset_id(&asset_id);
		self.paginated_request_all(&format!("assets/{asset_id_str}/addresses")).await
	}

	async fn blocks_latest(&self) -> Result<BlockContent, DataSourceError> {
		self.request("blocks/latest").await
	}

	async fn blocks_by_id(
		&self,
		id: impl Into<McBlockId> + Send,
	) -> Result<BlockContent, DataSourceError> {
		let id: McBlockId = id.into();
		self.request(&format!("blocks/{id}")).await
	}

	async fn blocks_slot(
		&self,
		slot_number: McSlotNumber,
	) -> Result<BlockContent, DataSourceError> {
		self.request(&format!("blocks/slot/{slot_number}")).await
	}

	async fn blocks_next(
		&self,
		id: impl Into<McBlockId> + Send,
	) -> Result<Vec<BlockContent>, DataSourceError> {
		let id: McBlockId = id.into();
		self.request(&format!("blocks/{id}/next")).await
	}

	async fn blocks_txs(
		&self,
		id: impl Into<McBlockId> + Send,
	) -> Result<Vec<String>, DataSourceError> {
		let id: McBlockId = id.into();
		self.request(&format!("blocks/{id}/txs")).await
	}

	async fn epochs_blocks(
		&self,
		epoch_number: McEpochNumber,
	) -> Result<Vec<String>, DataSourceError> {
		self.paginated_request_all(&format!("epochs/{epoch_number}/blocks")).await
	}
	async fn epochs_parameters(
		&self,
		epoch_number: McEpochNumber,
	) -> Result<EpochParamContent, DataSourceError> {
		self.request(&format!("epochs/{epoch_number}/parameters")).await
	}
	async fn epochs_stakes_by_pool(
		&self,
		epoch_number: McEpochNumber,
		pool_id: impl Into<McPoolId> + Send,
	) -> Result<Vec<EpochStakePoolContentInner>, DataSourceError> {
		let pool_id: McPoolId = pool_id.into();
		self.paginated_request_all(&format!("epochs/{epoch_number}/stakes/{pool_id}"))
			.await
	}

	async fn pools_history(
		&self,
		pool_id: impl Into<McPoolId> + Send,
	) -> Result<Vec<PoolHistoryInner>, DataSourceError> {
		let pool_id: McPoolId = pool_id.into();
		self.paginated_request_all(&format!("pools/{pool_id}/history")).await
	}
	async fn pools_extended(&self) -> Result<Vec<PoolListExtendedInner>, DataSourceError> {
		self.paginated_request_all("pools/extended").await
	}

	async fn scripts_datum_hash(
		&self,
		datum_hash: &str,
	) -> Result<Vec<serde_json::Value>, DataSourceError> {
		self.request(&format!("scripts/datum/{datum_hash}")).await
	}

	async fn transaction_by_hash(&self, tx_hash: McTxHash) -> Result<TxContent, DataSourceError> {
		self.request(&format!("txs/{tx_hash}")).await
	}

	async fn transactions_utxos(
		&self,
		tx_hash: McTxHash,
	) -> Result<TxContentUtxo, DataSourceError> {
		self.request(&format!("txs/{tx_hash}/utxos")).await
	}

	async fn genesis(&self) -> Result<GenesisContent, DataSourceError> {
		self.request("genesis").await
	}
}

pub fn format_asset_id(asset_id: &AssetId) -> String {
	let AssetId { policy_id, asset_name } = asset_id;
	format!("{}{}", &policy_id.to_hex_string()[2..], &asset_name.to_hex_string()[2..])
}

#[derive(Clone)]
#[allow(dead_code)]
enum Order {
	Asc,
	Desc,
}

impl std::fmt::Display for Order {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Order::Asc => write!(f, "asc"),
			Order::Desc => write!(f, "desc"),
		}
	}
}

#[derive(Clone)]
struct Pagination {
	count: i32,
	page: i32,
	order: Order,
	from: Option<String>,
	to: Option<String>,
}

impl Default for Pagination {
	fn default() -> Self {
		Self { count: 100, page: 1, order: Order::Asc, from: None, to: None }
	}
}
