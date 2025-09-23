#![recursion_limit = "256"]

mod cli_args;
mod client;
mod lean_server;
mod messages;
mod process;
mod server;
mod session;
mod session_set;

use anyhow::Error;
use clap::Parser;

use crate::cli_args::CliArgs;

#[tokio::main]
async fn main() -> Result<(), Error> {
  CliArgs::parse().run().await
}
