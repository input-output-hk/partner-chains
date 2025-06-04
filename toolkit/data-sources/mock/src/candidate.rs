use crate::Result;
use async_trait::async_trait;
use authority_selection_inherents::authority_selection_inputs::*;
use hex_literal::hex;
use log::{debug, info};
use serde::*;
use sidechain_domain::byte_string::*;
use sidechain_domain::*;

#[derive(Deserialize, Debug, Clone)]
pub struct MockRegistration {
	pub name: Option<String>,
	pub sidechain_pub_key: ByteString,
	pub mainchain_pub_key: ByteString,
	pub mainchain_signature: ByteString,
	pub sidechain_signature: ByteString,
	pub registration_utxo: UtxoId,
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
		let stake_pool_public_key = StakePoolPublicKey(mock.mainchain_pub_key.0.try_into().expect(
			"Invalid mock configuration. 'mainchain_pub_key' public key should be 32 bytes.",
		));
		let registrations = vec![RegistrationData {
			registration_utxo: mock.registration_utxo,
			sidechain_signature: SidechainSignature(mock.sidechain_signature.0.clone()),
			mainchain_signature: MainchainSignature(
				mock.mainchain_signature.0.try_into().expect("Mainchain signature is 64 bytes"),
			),
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
			tx_inputs: vec![mock.registration_utxo],
			aura_pub_key: AuraPublicKey(mock.aura_pub_key.0),
			grandpa_pub_key: GrandpaPublicKey(mock.grandpa_pub_key.0),
		}];
		let stake_delegation = Some(StakeDelegation(333));
		CandidateRegistrations { stake_pool_public_key, registrations, stake_delegation }
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

/// Mock authority selection data for a single epoch
#[derive(Deserialize, Clone, Debug)]
pub struct MockEpochCandidates {
	/// Permissioned candidates
	pub permissioned: Vec<MockPermissionedCandidate>,
	/// Active registrations (including invalid ones)
	pub registrations: Vec<MockRegistration>,
	/// Epoch nonce
	pub nonce: ByteString,
	/// Ariadne D-Parameter
	pub d_parameter: MockDParam,
}

/// Configuration of the mocked authority selection data source
pub struct MockRegistrationsConfig {
	/// List of epoch configurations
	/// These are returned for each epoch in a round-robin fashion
	pub epoch_rotation: Vec<MockEpochCandidates>,
}

impl MockRegistrationsConfig {
	/// Reads the mocked authority selection data from the file indicated by the
	/// `MOCK_REGISTRATIONS_FILE` environment variable.
	pub fn read() -> Result<MockRegistrationsConfig> {
		let registrations_file_path = std::env::var("MOCK_REGISTRATIONS_FILE")?;
		let registrations_config = Self::read_registrations(&registrations_file_path)?;
		Ok(registrations_config)
	}

	/// Reads the mocked authority selection data from file.
	pub fn read_registrations(path: &str) -> Result<MockRegistrationsConfig> {
		info!("Reading registrations from: {path}");
		let file = std::fs::File::open(path)?;
		let epoch_rotation: Vec<MockEpochCandidates> = serde_json::from_reader(file)?;
		info!("Loaded {} registration rotations", epoch_rotation.len());
		Ok(MockRegistrationsConfig { epoch_rotation })
	}
}

/// Mock authority selection data source that serves registration data in a round-robin fashion
///
/// # Creatin the data source
///
/// This data source can be created by wrapping a manually created [MockRegistrationsConfig].
/// However, the preferred way to do it is by loading the registrations data from a Json file
/// using the [MockRegistrationsConfig::read_registrations] method.
///
/// An example configuration file can look like this:
/// ```json
#[doc = include_str!("../examples/registrations.json")]
/// ```
///
/// See the structure and documentation of [MockEpochCandidates] for more information.
///
/// This file can be loaded and used to create a data source like this:
///
/// ```rust
/// # use std::io::Write;
/// # use std::fs::File;
/// # write!(File::create("registrations.json").unwrap(), "{}", include_str!("../examples/registrations.json"));
///
/// use partner_chains_mock_data_sources::*;
///
/// let registrations_data = MockRegistrationsConfig::read_registrations("registrations.json").unwrap();
///
/// let data_source = AuthoritySelectionDataSourceMock { registrations_data };
/// ```
pub struct AuthoritySelectionDataSourceMock {
	/// Data source configuration containing the mock data to be served
	pub registrations_data: MockRegistrationsConfig,
}

impl AuthoritySelectionDataSourceMock {
	pub(crate) fn epoch_data(&self, epoch_number: u32) -> MockEpochCandidates {
		let rotation_no: usize =
			epoch_number as usize % (self.registrations_data.epoch_rotation.len());
		self.registrations_data.epoch_rotation[rotation_no].clone()
	}

	/// Creates a new mocked authority selection data source using configuration from th
	/// file pointed to by the `MOCK_REGISTRATIONS_FILE` environment variable.
	pub fn new_from_env() -> Result<Self> {
		let registrations_data = MockRegistrationsConfig::read()?;
		Ok(AuthoritySelectionDataSourceMock { registrations_data })
	}
}

#[async_trait]
impl AuthoritySelectionDataSource for AuthoritySelectionDataSourceMock {
	async fn get_ariadne_parameters(
		&self,
		epoch_number: McEpochNumber,
		_d_parameter_validator: PolicyId,
		_permissioned_candidates_validator: PolicyId,
	) -> Result<AriadneParameters> {
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

		let permissioned_candidates: Option<Vec<RawPermissionedCandidateData>> =
			Some(candidates.into_iter().map(|p| p.into()).collect());

		Ok(AriadneParameters { d_parameter: d_parameter.into(), permissioned_candidates })
	}

	async fn get_candidates(
		&self,
		epoch: McEpochNumber,
		_committee_candidate_address: MainchainAddress,
	) -> Result<Vec<CandidateRegistrations>> {
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

	async fn get_epoch_nonce(&self, epoch_number: McEpochNumber) -> Result<Option<EpochNonce>> {
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

	async fn data_epoch(&self, for_epoch: McEpochNumber) -> Result<McEpochNumber> {
		Ok(McEpochNumber(for_epoch.0 - 2))
	}
}
