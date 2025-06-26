#![cfg(test)]
/// Experiment porting our pytest E2E tests into Rust
///
/// To run, start local-env and run `cargo test --bin e2e`
///
/// For now I've implemented TestDepositFunds from e2e-tests/tests/reserve/test_reserve_management.py
///
/// I tried to follow the structure of the Python code to a certain extent.
/// Fixtures are kept to their own functions but are passed by hand.
/// Tried to keep things simple for this initial prototype.
/// One thing to consider is to abandon the pythonic classy approach to CardanoCli and instead have command
/// calls as regular functions taking a context object containing all config data.
/// We might also want to not run the e2e tests through `cargo test` for better configurability and logging.
///
/// Rust crate `rstest` seems to be the standard for test fixtures in Rust, but I didn't feel the need
/// to start using it yet.
mod apiconfig;
mod blockchain_api;
mod conftest;
mod reserve;
mod run_command;
