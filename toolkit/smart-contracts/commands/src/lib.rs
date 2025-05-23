//! This crate provides types and functions that can be used to create a CLI
//! for managing the mainchain smart contracts relevant for a given partner
//! chain instance.
//!
//! ## Common Arguments
//!
//! Most type commands (usualy ending in "Cmd") take a [CommonArguments]
//! struct as argument. It stores the information neccessary for connecting
//! to the Ogmios server and retrying the operations like checking if a transaction
//! is included in the blockchain.
//!
//! ## Subcommands
//!
//! Each subcommand has its own command type, which implements the [clap::Parser]
//! trait. Each command type also has a `execute` method, which is used to execute
//! the command.
//!
//! Subcommands can execute transactions on the mainchain, query the mainchain
//! and also provide other utilities for managing the smart contracts.
//!
//! ## Result types
//!
//! Most commands return a [SubCmdResult] type, which is a wrapper around a
//! [serde_json::Value]. The returned value is printed to the ouptut at the end
//! of the command execution.
use ogmios_client::jsonrpsee::{OgmiosClients, client_for_url};
use partner_chains_cardano_offchain::{
	await_tx::FixedDelayRetries,
	cardano_keys::{CardanoKeyFileContent, CardanoPaymentSigningKey},
	multisig::MultiSigSmartContractResult,
};
use serde::Serialize;
use sidechain_domain::*;
use std::time::Duration;

pub mod assemble_tx;
pub mod d_parameter;
pub mod get_scripts;
pub mod governance;
pub mod governed_map;
pub mod permissioned_candidates;
pub mod register;
pub mod reserve;
pub mod sign_tx;

#[derive(Clone, Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
/// Commands for managing the mainchain smart contracts
pub enum SmartContractsCmd {
	/// Prints validator addresses and policy IDs of Partner Chain smart contracts
	GetScripts(get_scripts::GetScripts),
	/// Upsert DParameter
	UpsertDParameter(d_parameter::UpsertDParameterCmd),
	/// Upsert Permissioned Candidates
	UpsertPermissionedCandidates(permissioned_candidates::UpsertPermissionedCandidatesCmd),
	/// Register candidate
	Register(register::RegisterCmd),
	/// Deregister candidate
	Deregister(register::DeregisterCmd),
	#[command(subcommand)]
	/// Commands for management of rewards reserve
	Reserve(reserve::ReserveCmd),
	#[command(subcommand)]
	/// Commands for management of on-chain governance
	Governance(governance::GovernanceCmd),
	/// Assemble and submit a transaction
	AssembleAndSubmitTx(assemble_tx::AssembleAndSubmitCmd),
	/// Sign a transaction CBOR using a payment signing key
	SignTx(sign_tx::SignTxCmd),
	#[command(subcommand)]
	/// Manage the Governed Map key-value store on Cardano
	GovernedMap(governed_map::GovernedMapCmd),
}

#[derive(Clone, Debug, clap::Parser)]
#[command(author, version, about, long_about = None)]
/// Common command arguments
pub struct CommonArguments {
	#[arg(default_value = "ws://localhost:1337", long, short = 'O', env)]
	/// URL of the Ogmios server
	ogmios_url: String,
	#[arg(default_value = "180", long, env)]
	/// Timeout in seconds for Ogmios requests.
	ogmios_requests_timeout_seconds: u64,
	#[arg(default_value = "5", long)]
	/// Delay between retries in seconds. System will wait this long between
	/// queries checking if transaction is included in the blockchain.
	retry_delay_seconds: u64,
	#[arg(default_value = "59", long)]
	/// Number of retries. After transaction is submitted, system will try to check
	/// if it's included in the blockchain this many times.
	retry_count: usize,
}

impl CommonArguments {
	/// Connects to the Ogmios server and returns a client
	pub async fn get_ogmios_client(&self) -> crate::CmdResult<OgmiosClients> {
		Ok(client_for_url(
			&self.ogmios_url,
			Some(Duration::from_secs(self.ogmios_requests_timeout_seconds)),
		)
		.await
		.map_err(|e| format!("Failed to connect to Ogmios at {} with: {}", &self.ogmios_url, e))?)
	}

	/// Builds a `FixedDelayRetries` instance for retrying failed operations
	pub fn retries(&self) -> FixedDelayRetries {
		FixedDelayRetries::new(Duration::from_secs(self.retry_delay_seconds), self.retry_count)
	}
}

/// Result type for commands
type CmdResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
/// Result type for subcommands
type SubCmdResult = Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>;

impl SmartContractsCmd {
	/// Executes the internal command, and prints the result
	pub async fn execute(self) -> CmdResult<()> {
		let result: serde_json::Value = match self {
			Self::Governance(cmd) => cmd.execute().await,
			Self::GetScripts(cmd) => cmd.execute().await,
			Self::UpsertDParameter(cmd) => cmd.execute().await,
			Self::UpsertPermissionedCandidates(cmd) => cmd.execute().await,
			Self::Register(cmd) => cmd.execute().await,
			Self::Deregister(cmd) => cmd.execute().await,
			Self::Reserve(cmd) => cmd.execute().await,
			Self::AssembleAndSubmitTx(cmd) => cmd.execute().await,
			Self::SignTx(cmd) => cmd.execute().await,
			Self::GovernedMap(cmd) => cmd.execute().await,
		}?;
		println!("{}", result);
		Ok(())
	}

	/// Executes the internal command in a blocking manner
	pub fn execute_blocking(self) -> CmdResult<()> {
		tokio::runtime::Runtime::new()?.block_on(self.execute())
	}
}

/// Make a JSON object for a transaction hash. By default [McTxHash] is serialized
/// to a JSONString.
pub(crate) fn transaction_submitted_json(tx_hash: McTxHash) -> serde_json::Value {
	serde_json::json!(MultiSigSmartContractResult::TransactionSubmitted(tx_hash))
}

/// Converts an optional value to a JSON object. None values are converted to an empty object.
pub(crate) fn option_to_json<T: Serialize>(value_opt: Option<T>) -> serde_json::Value {
	match value_opt {
		Some(value) => serde_json::json!(value),
		None => serde_json::json!({}),
	}
}

#[derive(Clone, Debug, clap::Parser)]
pub(crate) struct PaymentFilePath {
	#[arg(long, short = 'k')]
	/// Path to the Cardano Signing Key file used to sign transaction(s) and pay for them
	payment_key_file: String,
}

impl PaymentFilePath {
	/// Reads the Cardano Signing Key file from the given path and returns a [CardanoPaymentSigningKey]
	pub(crate) fn read_key(&self) -> CmdResult<CardanoPaymentSigningKey> {
		let key_file = CardanoKeyFileContent::parse_file(&self.payment_key_file)?;
		Ok(CardanoPaymentSigningKey::try_from(key_file)?)
	}
}

#[derive(Clone, Debug, clap::Parser)]
pub(crate) struct GenesisUtxo {
	#[arg(long, short = 'c')]
	/// Genesis UTXO that identifies the partner chain
	genesis_utxo: UtxoId,
}

impl From<GenesisUtxo> for UtxoId {
	fn from(value: GenesisUtxo) -> Self {
		value.genesis_utxo
	}
}

// Parses public keys in formatted as SIDECHAIN_KEY:AURA_KEY:GRANDPA_KEY
pub(crate) fn parse_partnerchain_public_keys(
	partner_chain_public_keys: &str,
) -> CmdResult<PermissionedCandidateData> {
	let partner_chain_public_keys = partner_chain_public_keys.replace("0x", "");
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

#[cfg(test)]
mod test {
	use crate::parse_partnerchain_public_keys;
	use hex_literal::hex;
	use sidechain_domain::{
		AuraPublicKey, GrandpaPublicKey, PermissionedCandidateData, SidechainPublicKey,
	};

	#[test]
	fn parse_partnerchain_public_keys_with_0x_prefix() {
		let input = "039799ff93d184146deacaa455dade51b13ed16f23cdad11d1ad6af20103391180:e85534c93315d60f808568d1dce5cb9e8ba6ed0b204209c5cc8f3bec56c10b73:cdf3e5b33f53c8b541bbaea383225c45654f24de38c585725f3cff25b2802f55";
		assert_eq!(parse_partnerchain_public_keys(input).unwrap(), expected_public_keys())
	}

	#[test]
	fn parse_partnerchain_public_keys_without_0x_prefix() {
		let input = "0x039799ff93d184146deacaa455dade51b13ed16f23cdad11d1ad6af20103391180:0xe85534c93315d60f808568d1dce5cb9e8ba6ed0b204209c5cc8f3bec56c10b73:0xcdf3e5b33f53c8b541bbaea383225c45654f24de38c585725f3cff25b2802f55";
		assert_eq!(parse_partnerchain_public_keys(input).unwrap(), expected_public_keys())
	}

	fn expected_public_keys() -> PermissionedCandidateData {
		PermissionedCandidateData {
			sidechain_public_key: SidechainPublicKey(
				hex!("039799ff93d184146deacaa455dade51b13ed16f23cdad11d1ad6af20103391180").to_vec(),
			),
			aura_public_key: AuraPublicKey(
				hex!("e85534c93315d60f808568d1dce5cb9e8ba6ed0b204209c5cc8f3bec56c10b73").to_vec(),
			),
			grandpa_public_key: GrandpaPublicKey(
				hex!("cdf3e5b33f53c8b541bbaea383225c45654f24de38c585725f3cff25b2802f55").to_vec(),
			),
		}
	}
}
