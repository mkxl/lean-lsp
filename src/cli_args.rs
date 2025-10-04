use std::io::StdoutLock;

use anyhow::Error;
use clap::{Args, Parser, Subcommand};
use mkutils::{Tracing, Utils};
use tracing_subscriber::filter::LevelFilter;
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
  tracing_level_filter: LevelFilter,

  #[arg(long = "no-log-json")]
  tracing_json_disabled: bool,

  #[arg(long = "tokio-console-port", default_value_t = Tracing::DEFAULT_TOKIO_CONSOLE_PORT)]
  tracing_tokio_console_port: u16,

  #[arg(long = "tokio-console")]
  tracing_tokio_console_enabled: bool,

  #[command(subcommand)]
  command: Command,
}

impl CliArgs {
  const LOG_LEVEL_ENV_NAME: &'static str = "LOG_LEVEL";

  fn log_writer() -> StdoutLock<'static> {
    std::io::stdout().lock()
  }

  fn init_tracing(&self) {
    Tracing::default()
      .with_level_filter(self.tracing_level_filter)
      .with_json_enabled(!self.tracing_json_disabled)
      .with_tokio_console_port(self.tracing_tokio_console_port)
      .with_tokio_console_enabled(self.tracing_tokio_console_enabled)
      .with_writer(Self::log_writer)
      .init();
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
