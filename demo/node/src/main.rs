//! Substrate Node Template CLI library.
#![warn(missing_docs)]
#![allow(deprecated)]

mod chain_spec;
mod cli;
mod command;
mod data_sources;
mod inherent_data;
mod rpc;
mod service;
mod staging;
mod template_chain_spec;
mod testnet;

fn main() -> sc_cli::Result<()> {
	command::run()
}
