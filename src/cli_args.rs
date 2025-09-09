use std::{io::StdoutLock, net::IpAddr, path::PathBuf};

use anyhow::Error;
use clap::{Args, Parser, Subcommand};
use console_subscriber::{ConsoleLayer, Server as ConsoleServer};
use tracing_subscriber::{
  Layer, filter::LevelFilter, fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt,
};

use crate::{lean_server::LeanServer, session_set::SessionSet, utils::Utils};

#[derive(Args)]
struct Serve {
  #[arg(long, default_value_t = Self::DEFAULT_PORT)]
  port: u16,
}

impl Serve {
  const DEFAULT_PORT: u16 = 8080;

  async fn run(self) -> Result<(), Error> {
    std::future::pending().await
  }
}

#[derive(Args)]
struct Run {
  #[arg(default_value = Self::DEFAULT_LEAN_PATH_STR)]
  lean_path: PathBuf,

  #[arg(long = "log-dir", env = Self::LEAN_SERVER_LOG_DIRPATH_ENV_NAME)]
  lean_server_log_dirpath: Option<PathBuf>,
}

impl Run {
  const DEFAULT_LEAN_PATH_STR: &'static str = ".";
  const LEAN_SERVER_LOG_DIRPATH_ENV_NAME: &'static str = LeanServer::LOG_DIRPATH_ENV_NAME;

  async fn run(self) -> Result<(), Error> {
    SessionSet::run_session(self.lean_path, self.lean_server_log_dirpath).await
  }
}

#[derive(Subcommand)]
enum Command {
  Serve(Serve),
  Run(Run),
}

#[derive(Parser)]
pub struct CliArgs {
  #[arg(long = "log-level", default_value_t = LevelFilter::INFO, env = Self::LOG_LEVEL_ENV_NAME)]
  log_level_filter: LevelFilter,

  #[arg(long = "tokio-console")]
  #[allow(clippy::option_option)]
  tokio_console_port: Option<Option<u16>>,

  #[command(subcommand)]
  command: Command,
}

impl CliArgs {
  const DEFAULT_TOKIO_CONSOLE_PORT: u16 = ConsoleServer::DEFAULT_PORT;
  const DEFAULT_TOKIO_CONSOLE_IP_ADDR: IpAddr = ConsoleServer::DEFAULT_IP;
  const LOG_LEVEL_ENV_NAME: &'static str = "LOG_LEVEL";

  fn log_writer() -> StdoutLock<'static> {
    std::io::stdout().lock()
  }

  fn tracing_span_events() -> FmtSpan {
    FmtSpan::NEW | FmtSpan::CLOSE
  }

  fn init_tracing(&self) -> Result<(), Error> {
    let log_layer = tracing_subscriber::fmt::layer()
      .with_span_events(Self::tracing_span_events())
      .with_writer(Self::log_writer)
      .json()
      .with_filter(self.log_level_filter);
    let registry = tracing_subscriber::registry().with(log_layer);

    if let Some(tokio_console_server_port_opt) = self.tokio_console_port {
      let tokio_console_server_port = tokio_console_server_port_opt.unwrap_or(Self::DEFAULT_TOKIO_CONSOLE_PORT);
      let tokio_console_server_addr = (Self::DEFAULT_TOKIO_CONSOLE_IP_ADDR, tokio_console_server_port);
      let console_layer = ConsoleLayer::builder().server_addr(tokio_console_server_addr).spawn();

      registry.with(console_layer).init();
    } else {
      registry.init();
    }

    ().ok()
  }

  pub async fn run(self) -> Result<(), Error> {
    self.init_tracing()?;

    match self.command {
      Command::Serve(serve) => serve.run().await,
      Command::Run(run) => run.run().await,
    }
  }
}
