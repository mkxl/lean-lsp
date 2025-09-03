mod cli_args;
mod lean_server;
mod process;
mod session;
mod utils;

use anyhow::Error;
use clap::Parser;

use crate::cli_args::CliArgs;

#[tokio::main]
async fn main() -> Result<(), Error> {
  CliArgs::parse().run().await
}
