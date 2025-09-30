use std::{io::StdoutLock, net::IpAddr};

use anyhow::Error;
use clap::{Args, Parser, Subcommand};
use console_subscriber::{ConsoleLayer, Server as ConsoleServer};
use mkutils::Utils;
use tracing_subscriber::{
  Layer, filter::LevelFilter, fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt,
};
use ulid::Ulid;

use crate::{
  client::Client,
  server::Server,
  session_set::{NewSessionCommand, OpenFileCommand},
};

#[derive(Args)]
struct Get {
  #[arg(long, default_value_t = Server::DEFAULT_PORT)]
  port: u16,

  session_id: Option<Ulid>,
}

impl Get {
  async fn run(self) -> Result<(), Error> {
    Client::new(self.port)?
      .get(self.session_id)
      .await?
      .session_ids
      .iter()
      .for_each(Utils::println)
      .ok()
  }
}

#[derive(Args)]
struct New {
  #[arg(long, default_value_t = Server::DEFAULT_PORT)]
  port: u16,

  #[command(flatten)]
  command: NewSessionCommand,
}

impl New {
  async fn run(self) -> Result<(), Error> {
    Client::new(self.port)?
      .new_session(&self.command)
      .await?
      .session_id
      .println()
      .ok()
  }
}

#[derive(Args)]
struct Open {
  #[arg(long, default_value_t = Server::DEFAULT_PORT)]
  port: u16,

  #[command(flatten)]
  command: OpenFileCommand,
}

impl Open {
  async fn run(self) -> Result<(), Error> {
    Client::new(self.port)?.open_file(&self.command).await?.ok()
  }
}

#[derive(Args)]
struct Serve {
  #[arg(long, default_value_t = Server::DEFAULT_PORT)]
  port: u16,
}

impl Serve {
  async fn run(self) -> Result<(), Error> {
    Server::serve(self.port).await
  }
}

#[derive(Subcommand)]
enum Command {
  Get(Get),
  New(New),
  Open(Open),
  Run(NewSessionCommand),
  Serve(Serve),
}

#[derive(Parser)]
pub struct CliArgs {
  #[arg(long = "log-level", default_value_t = LevelFilter::INFO, env = Self::LOG_LEVEL_ENV_NAME)]
  log_level_filter: LevelFilter,

  #[arg(long = "tokio-console", require_equals = true)]
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

  fn init_tracing(&self) {
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
  }

  pub async fn run(self) -> Result<(), Error> {
    self.init_tracing();

    match self.command {
      Command::Get(get) => get.run().await,
      Command::New(new) => new.run().await,
      Command::Open(open) => open.run().await,
      Command::Run(new_session_command) => new_session_command.run().await,
      Command::Serve(serve) => serve.run().await,
    }
  }
}
