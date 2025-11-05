#![recursion_limit = "256"]

mod cli_args;
mod client;
mod commands;
mod lean_server;
mod macros;
mod messages;
mod server;
mod session;
mod session_runner;
mod session_set;
mod session_set_runner;
mod types;

use anyhow::Error as AnyhowError;
use clap::Parser;

use crate::cli_args::CliArgs;

#[tokio::main]
async fn main() -> Result<(), AnyhowError> {
  CliArgs::parse().run().await
}
