//! Substrate Node Template CLI library.
#![warn(missing_docs)]

#[macro_use]
mod benchmarking;
mod chain_init;
mod chain_spec;
mod cli;
mod command;
mod inherent_data;
mod main_chain_follower;
mod rpc;
mod service;
mod staging;
mod template_chain_spec;
mod testnet;

fn main() -> sc_cli::Result<()> {
	command::run()
}
