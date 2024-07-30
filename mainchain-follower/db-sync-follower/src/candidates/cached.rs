//! Provides caches for call which results are immutable,
//! like epoch candidates queries made by inherent data producers.
//! Also provides a way of batching and caching incoming transactions,
//! to allow efficient queries when syncing the chain.

use crate::candidates::CandidatesDataSourceImpl;
use async_trait::async_trait;
use figment::{providers::Env, Figment};
use log::info;
use lru::LruCache;
use main_chain_follower_api::{candidate::*, Result};
use serde::Deserialize;
use sidechain_domain::*;
use std::{
	error::Error,
	sync::{Arc, Mutex},
};

pub type ArcMut<T> = Arc<Mutex<T>>;

type AriadneParametersCacheKey = (McEpochNumber, PolicyId, PolicyId);
type CandidatesCacheKey = (McEpochNumber, String);
pub struct CandidateDataSourceCached {
	inner: CandidatesDataSourceImpl,
	get_ariadne_parameters_for_epoch_cache:
		ArcMut<LruCache<AriadneParametersCacheKey, AriadneParameters>>,
	get_candidates_for_epoch_cache:
		ArcMut<LruCache<CandidatesCacheKey, Vec<CandidateRegistrations>>>,
	security_parameter: u32,
	highest_seen_stable_epoch: ArcMut<Option<McEpochNumber>>,
}

#[async_trait]
impl CandidateDataSource for CandidateDataSourceCached {
	async fn get_ariadne_parameters(
		&self,
		epoch: McEpochNumber,
		d_parameter_policy: PolicyId,
		permissioned_candidates_policy: PolicyId,
	) -> Result<AriadneParameters> {
		if self.can_use_caching_for_request(epoch).await? {
			self.get_ariadne_parameters_with_caching(
				epoch,
				d_parameter_policy,
				permissioned_candidates_policy,
			)
			.await
		} else {
			self.inner
				.get_ariadne_parameters(epoch, d_parameter_policy, permissioned_candidates_policy)
				.await
		}
	}

	async fn get_candidates(
		&self,
		epoch: McEpochNumber,
		committee_candidate_address: MainchainAddress,
	) -> Result<Vec<CandidateRegistrations>> {
		if self.can_use_caching_for_request(epoch).await? {
			self.get_candidates_with_caching(epoch, committee_candidate_address).await
		} else {
			self.inner.get_candidates(epoch, committee_candidate_address).await
		}
	}

	async fn get_epoch_nonce(&self, epoch: McEpochNumber) -> Result<Option<EpochNonce>> {
		self.inner.get_epoch_nonce(epoch).await
	}

	async fn data_epoch(&self, for_epoch: McEpochNumber) -> Result<McEpochNumber> {
		self.inner.data_epoch(for_epoch).await
	}
}

#[derive(Debug, Clone, Deserialize)]
pub struct CandidateDataSourceCacheConfig {
	pub cardano_security_parameter: u32,
}

impl CandidateDataSourceCacheConfig {
	pub fn from_env() -> std::result::Result<Self, Box<dyn Error + Send + Sync + 'static>> {
		let config: Self = Figment::new()
			.merge(Env::raw())
			.extract()
			.map_err(|e| format!("Failed to read candidates data source config: {e}"))?;
		info!("Using candidate data source configuration: {config:?}");
		Ok(config)
	}
}

impl CandidateDataSourceCached {
	pub fn new(
		inner: CandidatesDataSourceImpl,
		candidates_for_epoch_cache_size: usize,
		security_parameter: u32,
	) -> Self {
		Self {
			inner,
			get_ariadne_parameters_for_epoch_cache: Arc::new(Mutex::new(LruCache::new(
				candidates_for_epoch_cache_size.try_into().unwrap(),
			))),
			get_candidates_for_epoch_cache: Arc::new(Mutex::new(LruCache::new(
				candidates_for_epoch_cache_size.try_into().unwrap(),
			))),
			security_parameter,
			highest_seen_stable_epoch: Arc::new(Mutex::new(None)),
		}
	}
	pub fn new_from_env(
		inner: CandidatesDataSourceImpl,
		candidates_for_epoch_cache_size: usize,
	) -> std::result::Result<Self, Box<dyn Error + Send + Sync + 'static>> {
		let config = CandidateDataSourceCacheConfig::from_env()?;
		Ok(Self::new(inner, candidates_for_epoch_cache_size, config.cardano_security_parameter))
	}

	async fn get_candidates_with_caching(
		&self,
		epoch: McEpochNumber,
		committee_candidate_address: MainchainAddress,
	) -> Result<Vec<CandidateRegistrations>> {
		log::debug!("get_candidates_with_caching({:?})", epoch.0);
		let key = (epoch, committee_candidate_address.to_string());
		if let Ok(mut cache) = self.get_candidates_for_epoch_cache.lock() {
			if let Some(resp) = cache.get(&key) {
				log::debug!("Serving cached candidates for epoch: {:?}", epoch.0);
				return Ok(resp.clone());
			}
		}

		let response = self.inner.get_candidates(epoch, committee_candidate_address).await?;
		if let Ok(mut cache) = self.get_candidates_for_epoch_cache.lock() {
			log::debug!("Caching candidates for epoch: {:?}", epoch.0);
			cache.put(key, response.clone());
		}
		Ok(response)
	}

	// Use only for stable epochs
	async fn get_ariadne_parameters_with_caching(
		&self,
		epoch: McEpochNumber,
		d_parameter_validator: PolicyId,
		permissioned_candidates_validator: PolicyId,
	) -> Result<AriadneParameters> {
		log::debug!("get_ariadne_parameters_with_caching({:?})", epoch.0);
		let key = (epoch, d_parameter_validator.clone(), permissioned_candidates_validator.clone());
		if let Ok(mut cache) = self.get_ariadne_parameters_for_epoch_cache.lock() {
			if let Some(resp) = cache.get(&key) {
				log::debug!("Serving cached ariadne parameters for epoch: {:?}", epoch.0);
				return Ok(resp.clone());
			}
		}

		let response = self
			.inner
			.get_ariadne_parameters(epoch, d_parameter_validator, permissioned_candidates_validator)
			.await?;
		if let Ok(mut cache) = self.get_ariadne_parameters_for_epoch_cache.lock() {
			log::debug!("Caching ariadne parameters for epoch: {:?}", epoch.0);
			cache.put(key, response.clone());
		}
		Ok(response)
	}

	async fn can_use_caching_for_request(&self, request_epoch: McEpochNumber) -> Result<bool> {
		let data_epoch = self.inner.data_epoch(request_epoch).await?;
		if let Ok(stable_epoch) = self.highest_seen_stable_epoch.lock() {
			if stable_epoch.map_or(false, |stable_epoch| stable_epoch >= data_epoch) {
				return Ok(true);
			}
		}
		match crate::db_model::get_latest_stable_epoch(&self.inner.pool, self.security_parameter)
			.await?
		{
			Some(stable_epoch) => {
				let stable_epoch = McEpochNumber(stable_epoch.0);
				if let Ok(mut highest_seen_stable_epoch) = self.highest_seen_stable_epoch.lock() {
					*highest_seen_stable_epoch = Some(stable_epoch);
				}
				Ok(data_epoch <= stable_epoch)
			},
			None => Ok(false),
		}
	}
}
