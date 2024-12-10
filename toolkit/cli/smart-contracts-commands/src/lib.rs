use sidechain_domain::*;

pub mod d_parameter;
pub mod get_scripts;
pub mod init_governance;
pub mod register;

#[derive(Clone, Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum SmartContractsCmd {
	/// Print validator addresses and policy IDs of Partner Chain smart contracts
	GetScripts(get_scripts::GetScripts),
	/// Initialize Partner Chain governance
	InitGovernance(init_governance::InitGovernanceCmd),
	/// Upsert DParameter
	UpsertDParameter(d_parameter::UpsertDParameterCmd),
	/// Register candidate
	Register(register::RegisterCmd),
}

#[derive(Clone, Debug, clap::Parser)]
#[command(author, version, about, long_about = None)]
pub struct CommonArguments {
	#[arg(default_value = "http://localhost:1337", long, short = 'O')]
	ogmios_url: String,
}

type CmdResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

impl SmartContractsCmd {
	pub async fn execute(self) -> CmdResult<()> {
		match self {
			Self::InitGovernance(cmd) => cmd.execute().await,
			Self::GetScripts(cmd) => cmd.execute().await,
			Self::UpsertDParameter(cmd) => cmd.execute().await,
			Self::Register(cmd) => cmd.execute().await,
		}
	}

	pub fn execute_blocking(self) -> CmdResult<()> {
		tokio::runtime::Runtime::new()?.block_on(self.execute())
	}
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CardanoKeyFileContent {
	cbor_hex: String,
}

pub(crate) fn read_private_key_from_file(path: &str) -> CmdResult<MainchainPrivateKey> {
	let file_content_str = String::from_utf8(std::fs::read(path)?)?;
	let file_content = serde_json::from_str::<CardanoKeyFileContent>(&file_content_str)?;
	let key_hex = (file_content.cbor_hex.strip_prefix("5820"))
		.ok_or("CBOR prefix missing in payment key".to_string())?;
	let key_bytes = (hex::decode(key_hex)?.try_into())
		.map_err(|_| format!("{} is not the valid lengh of 32", key_hex))?;
	Ok(MainchainPrivateKey(key_bytes))
}

// Parses public keys in formatted as SIDECHAIN_KEY:AURA_KEY:GRANDPA_KEY
pub(crate) fn parse_partnerchain_public_keys(
	partner_chain_public_keys: &str,
) -> CmdResult<PermissionedCandidateData> {
	if let [sidechain_pub_key, aura_pub_key, grandpa_pub_key] =
		partner_chain_public_keys.split(":").collect::<Vec<_>>()[..]
	{
		Ok(PermissionedCandidateData {
			sidechain_public_key: SidechainPublicKey(hex::decode(sidechain_pub_key)?),
			aura_public_key: AuraPublicKey(hex::decode(aura_pub_key)?),
			grandpa_public_key: GrandpaPublicKey(hex::decode(grandpa_pub_key)?),
		})
	} else {
		Err("Failed to parse partner chain public keys.".into())
	}
}

fn payment_signing_key_to_mainchain_address_hash(
	payment_signing_key: MainchainPrivateKey,
) -> CmdResult<MainchainAddressHash> {
	Ok(cardano_serialization_lib::PrivateKey::from_normal_bytes(&payment_signing_key.0)?
		.to_public()
		.hash()
		.to_bytes()
		.as_slice()
		.try_into()
		.map(MainchainAddressHash)?)
}
