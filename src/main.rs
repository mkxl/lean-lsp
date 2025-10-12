#![recursion_limit = "256"]

mod cli_args;
mod client;
mod commands;
mod lean_server;
mod messages;
mod server;
mod session;
mod session_runner;
mod session_set;
mod session_set_runner;

use anyhow::Error;
use clap::Parser;

use crate::cli_args::CliArgs;

#[tokio::main]
async fn main() -> Result<(), Error> {
  CliArgs::parse().run().await
}
