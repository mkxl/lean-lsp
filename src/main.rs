#![recursion_limit = "256"]
#![feature(exit_status_error)]

mod cli_args;
mod commands;
mod highlight_state;
mod info_view;
mod info_view_builder;
mod lean_server_process;
mod message;
mod notification;
mod open_file;
mod responses;
mod server;
mod session;
mod session_map;
mod tui;
mod tui_map;
mod types;
mod view;

use anyhow::Error as AnyhowError;
use clap::Parser;

use crate::cli_args::CliArgs;

#[tokio::main]
async fn main() -> Result<(), AnyhowError> {
  CliArgs::parse().run().await
}
