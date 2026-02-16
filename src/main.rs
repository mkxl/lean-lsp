#![recursion_limit = "256"]
#![feature(exit_status_error)]

mod cli_args;
mod commands;
mod lean_server_process;
mod message;
mod notification;
mod open_file;
mod render_state;
mod responses;
mod server;
mod session;
mod session_map;
mod tui;
mod tui_set;
mod types;

use anyhow::Error as AnyhowError;
use clap::Parser;

use crate::cli_args::CliArgs;

#[tokio::main]
async fn main() -> Result<(), AnyhowError> {
  CliArgs::parse().run().await
}
