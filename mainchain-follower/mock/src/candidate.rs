use async_trait::async_trait;
use authority_selection_inherents::authority_selection_inputs::*;
use hex_literal::hex;
use log::{debug, info};
use serde::*;
use sidechain_domain::byte_string::*;
use sidechain_domain::mainchain_epoch::MainchainEpochConfig;
use sidechain_domain::*;
use std::error::Error;

#[derive(Deserialize, Debug, Clone)]
pub struct MockRegistration {
	pub name: Option<String>,
	pub sidechain_pub_key: ByteString,
	pub mainchain_pub_key: ByteString,
	pub mainchain_signature: ByteString,
	pub sidechain_signature: ByteString,
	pub input_utxo: UtxoId,
	pub status: MockRegistrationStatus,
	pub aura_pub_key: ByteString,
	pub grandpa_pub_key: ByteString,
}

impl MockRegistration {
	/// Returns an info string like: "Bob(0x039...1f27, active)"
	pub fn info_string(&self) -> String {
		let name = self.name.clone().unwrap_or("<Unnamed>".into());
		let status = match self.status {
			MockRegistrationStatus::Active => "active".to_string(),
			MockRegistrationStatus::PendingActivation { effective_at } => {
				format!("active at {effective_at}")
			},
			MockRegistrationStatus::PendingDeregistration { effective_at } => {
				format!("active until {effective_at}")
			},
		};
		let mut short_addr = self.sidechain_pub_key.to_hex_string();
		short_addr.replace_range(5..(short_addr.len() - 4), "...");
		format!("{name}({short_addr}, {status})")
	}
}

impl From<MockRegistration> for CandidateRegistrations {
	fn from(mock: MockRegistration) -> Self {
		let mainchain_pub_key = MainchainPublicKey(mock.mainchain_pub_key.0.try_into().expect(
			"Invalid mock configuration. 'mainchain_pub_key' public key should be 32 bytes.",
		));
		let registrations = vec![RegistrationData {
			consumed_input: mock.input_utxo,
			sidechain_signature: SidechainSignature(mock.sidechain_signature.0.clone()),
			mainchain_signature: MainchainSignature(mock.mainchain_signature.0),
			cross_chain_signature: CrossChainSignature(mock.sidechain_signature.0.clone()),
			sidechain_pub_key: SidechainPublicKey(mock.sidechain_pub_key.0.clone()),
			cross_chain_pub_key: CrossChainPublicKey(mock.sidechain_pub_key.0.clone()),
			utxo_info: UtxoInfo {
				utxo_id: UtxoId {
					tx_hash: McTxHash(hex!(
						"5a9b57731df0e008c5aa7296482c033212b71a3c1796ff00c10db7150c1f3d1d"
					)),
					index: UtxoIndex(9),
				},
				epoch_number: McEpochNumber(123),
				block_number: McBlockNumber(12345),
				slot_number: McSlotNumber(123456),
				tx_index_within_block: McTxIndexInBlock(12),
			},
			tx_inputs: vec![mock.input_utxo],
			aura_pub_key: AuraPublicKey(mock.aura_pub_key.0),
			grandpa_pub_key: GrandpaPublicKey(mock.grandpa_pub_key.0),
		}];
		let stake_delegation = Some(StakeDelegation(333));
		CandidateRegistrations { mainchain_pub_key, registrations, stake_delegation }
	}
}

#[derive(Deserialize, Debug, Clone)]
pub enum MockRegistrationStatus {
	Active,
	PendingActivation { effective_at: u64 },
	PendingDeregistration { effective_at: u64 },
}

#[derive(Deserialize, Debug, Clone)]
pub struct MockPermissionedCandidate {
	name: Option<String>,
	sidechain_pub_key: ByteString,
	aura_pub_key: ByteString,
	grandpa_pub_key: ByteString,
}

impl MockPermissionedCandidate {
	/// Returns an info string like: Bob(0x039...1f27)
	pub fn info_string(&self) -> String {
		let name = self.clone().name.unwrap_or("<unnamed>".into());
		let mut short_addr = self.sidechain_pub_key.to_hex_string();
		short_addr.replace_range(5..(short_addr.len() - 4), "...");
		format!("{}({})", name, short_addr)
	}
}

impl From<MockPermissionedCandidate> for RawPermissionedCandidateData {
	fn from(
		MockPermissionedCandidate {
			name: _,
			sidechain_pub_key,
			aura_pub_key,
			grandpa_pub_key,
		}: MockPermissionedCandidate,
	) -> Self {
		Self {
			sidechain_public_key: SidechainPublicKey(sidechain_pub_key.0),
			aura_public_key: AuraPublicKey(aura_pub_key.0),
			grandpa_public_key: GrandpaPublicKey(grandpa_pub_key.0),
		}
	}
}

#[derive(Deserialize, Clone, Debug)]
pub struct MockDParam {
	permissioned: u16,
	registered: u16,
}

impl MockDParam {
	pub fn info_string(&self) -> String {
		format!("permissioned: {}, registered: {}", self.permissioned, self.registered)
	}
}

impl From<MockDParam> for DParameter {
	fn from(MockDParam { permissioned, registered }: MockDParam) -> Self {
		Self { num_permissioned_candidates: permissioned, num_registered_candidates: registered }
	}
}

#[derive(Deserialize, Clone, Debug)]
pub struct MockEpochCandidates {
	pub permissioned: Vec<MockPermissionedCandidate>,
	pub registrations: Vec<MockRegistration>,
	pub nonce: ByteString,
	pub d_parameter: MockDParam,
}

pub struct MockRegistrationsConfig {
	/// List of epoch configurations
	/// These are returned for each epoch in a round-robin fashion
	pub epoch_rotation: Vec<MockEpochCandidates>,
}

impl MockRegistrationsConfig {
	pub fn read(
	) -> std::result::Result<MockRegistrationsConfig, Box<dyn Error + Send + Sync + 'static>> {
		let registrations_file_path = std::env::var("MAIN_CHAIN_FOLLOWER_MOCK_REGISTRATIONS_FILE")?;
		let registrations_config = Self::read_registrations(registrations_file_path)?;
		Ok(registrations_config)
	}
	pub fn read_registrations(
		path: String,
	) -> std::result::Result<MockRegistrationsConfig, Box<dyn Error + Send + Sync + 'static>> {
		info!("Reading registrations from: {path}");
		let file = std::fs::File::open(path)?;
		let epoch_rotation: Vec<MockEpochCandidates> = serde_json::from_reader(file)?;
		info!("Loaded {} registration rotations", epoch_rotation.len());
		Ok(MockRegistrationsConfig { epoch_rotation })
	}
}

pub struct MockCandidateDataSource {
	pub registrations_data: MockRegistrationsConfig,
	pub mc_epoch_config: MainchainEpochConfig,
}

impl MockCandidateDataSource {
	pub fn epoch_data(&self, epoch_number: u32) -> MockEpochCandidates {
		let rotation_no: usize =
			epoch_number as usize % (self.registrations_data.epoch_rotation.len());
		self.registrations_data.epoch_rotation[rotation_no].clone()
	}

	pub fn new_from_env() -> std::result::Result<Self, Box<dyn Error + Send + Sync + 'static>> {
		let registrations_data = MockRegistrationsConfig::read()?;
		let mc_epoch_config = MainchainEpochConfig::read_from_env()?;
		Ok(MockCandidateDataSource { registrations_data, mc_epoch_config })
	}
}

#[async_trait]
impl AuthoritySelectionDataSource for MockCandidateDataSource {
	async fn get_ariadne_parameters(
		&self,
		epoch_number: McEpochNumber,
		_d_parameter_validator: PolicyId,
		_permissioned_candidates_validator: PolicyId,
	) -> Result<AriadneParameters, Box<dyn std::error::Error + Send + Sync>> {
		let epoch_number = epoch_number.0;
		debug!("Received get_d_parameter_for_epoch({epoch_number}) request");

		let d_parameter = self.epoch_data(epoch_number).d_parameter;
		debug!("    Responding with: {}", d_parameter.info_string());

		debug!("Received get_permissioned_candidates_for_epoch({epoch_number}) request");

		let candidates = self.epoch_data(epoch_number).permissioned;

		debug!(
			"    Responding with: {:?}",
			candidates.iter().cloned().map(|c| c.info_string()).collect::<Vec<_>>()
		);

		let permissioned_candidates: Vec<RawPermissionedCandidateData> =
			candidates.into_iter().map(|p| p.into()).collect();

		Ok(AriadneParameters { d_parameter: d_parameter.into(), permissioned_candidates })
	}

	async fn get_candidates(
		&self,
		epoch: McEpochNumber,
		_committee_candidate_address: MainchainAddress,
	) -> Result<Vec<CandidateRegistrations>, Box<dyn std::error::Error + Send + Sync>> {
		let epoch_number = epoch.0;
		debug!("Received get_candidates({epoch_number}) request");

		let epoch_conf = self.epoch_data(epoch_number);
		let registrations = epoch_conf.registrations;

		debug!(
			"    Responding with:
    Registrations: {:?}",
			registrations.iter().cloned().map(|r| r.info_string()).collect::<Vec<_>>()
		);
		Ok(registrations.into_iter().map(CandidateRegistrations::from).collect())
	}

	async fn get_epoch_nonce(
		&self,
		epoch_number: McEpochNumber,
	) -> Result<Option<EpochNonce>, Box<dyn std::error::Error + Send + Sync>> {
		let epoch_number = epoch_number.0;
		debug!("Received get_epoch_nonce({epoch_number}) request");
		let epoch_conf = self.epoch_data(epoch_number);
		debug!(
			"    Responding with:
    Nonce: {}",
			epoch_conf.nonce.to_hex_string(),
		);
		Ok(Some(EpochNonce(epoch_conf.nonce.clone().0)))
	}

	async fn data_epoch(
		&self,
		for_epoch: McEpochNumber,
	) -> Result<McEpochNumber, Box<dyn std::error::Error + Send + Sync>> {
		Ok(McEpochNumber(for_epoch.0 - 2))
	}
}
