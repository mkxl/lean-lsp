mod cli_args;
mod process;
mod session;
mod utils;

use crate::cli_args::CliArgs;
use anyhow::Error;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Error> {
  CliArgs::parse().run().await
}
