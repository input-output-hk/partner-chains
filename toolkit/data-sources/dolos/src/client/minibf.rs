use async_trait::async_trait;
use blockfrost_openapi::models::{
	address_transactions_content_inner::AddressTransactionsContentInner,
	address_utxo_content_inner::AddressUtxoContentInner,
	asset_addresses_inner::AssetAddressesInner, asset_transactions_inner::AssetTransactionsInner,
	block_content::BlockContent, epoch_param_content::EpochParamContent,
	epoch_stake_pool_content_inner::EpochStakePoolContentInner,
	pool_history_inner::PoolHistoryInner, pool_list_extended_inner::PoolListExtendedInner,
	tx_content::TxContent, tx_content_utxo::TxContentUtxo,
};
use serde::de::DeserializeOwned;
use sidechain_domain::*;
use std::time::Duration;
use ureq::Agent;

use crate::client::api::{McBlockId, MiniBFApi};

#[derive(Clone)]
pub struct MiniBFClient {
	agent: ureq::Agent,
	addr: String,
}

impl MiniBFClient {
	pub fn new(addr: &str, timeout: Duration) -> Self {
		let agent = Agent::config_builder().timeout_per_call(Some(timeout)).build().into();
		MiniBFClient { agent, addr: addr.to_string() }
	}

	pub async fn request<T: DeserializeOwned + std::fmt::Debug>(
		&self,
		method: &str,
	) -> Result<T, String> {
		let req = format!("{}/{}", self.addr, method);
		log::trace!("Dolos request: {req:?}");
		let resp = self
			.agent
			.get(req)
			.call()
			.map_err(|e| e.to_string())
			.and_then(|mut r| r.body_mut().read_json().map_err(|e| e.to_string()));
		log::trace!("Dolos response: {resp:?}");
		resp
	}

	pub async fn paginated_request<T: DeserializeOwned + std::fmt::Debug>(
		&self,
		method: &str,
		pagination: Pagination,
	) -> Result<Vec<T>, String> {
		let mut req = url::form_urlencoded::Serializer::new(format!("{}/{}", self.addr, method));
		req.extend_pairs([
			("count", &pagination.count.to_string()),
			("page", &pagination.page.to_string()),
			("order", &pagination.order.to_string()),
		]);
		if let Some(from) = pagination.from {
			req.append_pair("from", &from);
		}
		if let Some(to) = pagination.to {
			req.append_pair("to", &to);
		}
		let req_url = req.finish();
		log::trace!("Dolos request: {req_url:?}");
		let resp = self
			.agent
			.get(req_url)
			.call()
			.map_err(|e| e.to_string())
			.and_then(|mut r| r.body_mut().read_json().map_err(|e| e.to_string()));
		log::trace!("Dolos response: {resp:?}");
		resp
	}

	pub async fn paginated_request_all<T: DeserializeOwned + std::fmt::Debug>(
		&self,
		method: &str,
	) -> Result<Vec<T>, String> {
		let mut pagination: Pagination = Pagination::default();
		let mut have_all_pages = false;
		let mut res = Vec::new();
		while !have_all_pages {
			let mut resp: Vec<T> = self.paginated_request(method, pagination.clone()).await?;
			if resp.len() < 100 {
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
	) -> Result<Vec<AddressUtxoContentInner>, String> {
		self.paginated_request_all(&format!("/addresses/{address}/utxos")).await
	}

	async fn addresses_transactions(
		&self,
		address: MainchainAddress,
	) -> Result<Vec<AddressTransactionsContentInner>, String> {
		self.paginated_request_all(&format!("/addresses/{address}/transactions")).await
	}

	async fn assets_transactions(
		&self,
		asset_id: AssetId,
	) -> Result<Vec<AssetTransactionsInner>, String> {
		let AssetId { policy_id, asset_name } = asset_id;
		self.paginated_request_all(&format!("/assets/{policy_id}{asset_name}/transactions"))
			.await
	}

	async fn assets_addresses(
		&self,
		asset_id: AssetId,
	) -> Result<Vec<AssetAddressesInner>, String> {
		let AssetId { policy_id, asset_name } = asset_id;
		self.paginated_request_all(&format!("/assets/{policy_id}{asset_name}/addresses"))
			.await
	}

	async fn blocks_latest(&self) -> Result<BlockContent, String> {
		self.request("/blocks/latest").await
	}

	async fn blocks_by_id(&self, id: impl Into<McBlockId> + Send) -> Result<BlockContent, String> {
		let id: McBlockId = id.into();
		self.request(&format!("/blocks/{id}")).await
	}

	async fn blocks_slot(&self, slot_number: McSlotNumber) -> Result<BlockContent, String> {
		self.request(&format!("/blocks/slot/{slot_number}")).await
	}

	async fn blocks_next(
		&self,
		id: impl Into<McBlockId> + Send,
	) -> Result<Vec<BlockContent>, String> {
		let id: McBlockId = id.into();
		self.request(&format!("/blocks/{id}/next")).await
	}

	async fn blocks_txs(&self, id: impl Into<McBlockId> + Send) -> Result<Vec<String>, String> {
		let id: McBlockId = id.into();
		self.request(&format!("/blocks/{id}/txs")).await
	}

	async fn epochs_blocks(&self, epoch_number: McEpochNumber) -> Result<Vec<String>, String> {
		self.paginated_request_all(&format!("/epochs/{epoch_number}/blocks")).await
	}
	async fn epochs_parameters(
		&self,
		epoch_number: McEpochNumber,
	) -> Result<EpochParamContent, String> {
		self.request(&format!("/epochs/{epoch_number}/parameters")).await
	}
	async fn epochs_stakes_by_pool(
		&self,
		epoch_number: McEpochNumber,
		pool_id: &str,
	) -> Result<Vec<EpochStakePoolContentInner>, String> {
		self.paginated_request_all(&format!("/epochs/{epoch_number}/stakes/{pool_id}"))
			.await
	}

	async fn pools_history(&self, pool_id: &str) -> Result<Vec<PoolHistoryInner>, String> {
		self.paginated_request_all(&format!("/pools/{pool_id}/history")).await
	}
	async fn pools_extended(&self) -> Result<Vec<PoolListExtendedInner>, String> {
		self.paginated_request_all("/pools/extended").await
	}

	async fn scripts_datum_hash(&self, datum_hash: &str) -> Result<Vec<serde_json::Value>, String> {
		self.request(&format!("/scripts/datum/{datum_hash}")).await
	}

	async fn transaction_by_hash(&self, tx_hash: McTxHash) -> Result<TxContent, String> {
		self.request(&format!("/txs/{tx_hash}")).await
	}

	async fn transactions_utxos(&self, tx_hash: McTxHash) -> Result<TxContentUtxo, String> {
		self.request(&format!("/txs/{tx_hash}/utxos")).await
	}
}

#[derive(Clone)]
pub enum Order {
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
pub struct Pagination {
	pub count: i32,
	pub page: i32,
	pub order: Order,
	pub from: Option<String>,
	pub to: Option<String>,
}

impl Default for Pagination {
	fn default() -> Self {
		Self { count: 100, page: 1, order: Order::Asc, from: None, to: None }
	}
}
