use std::io::StdoutLock;

use anyhow::Error as AnyhowError;
use clap::{Args, Parser, Subcommand};
use futures::StreamExt;
use mkutils::{Tracing, Utils};
use tracing_subscriber::filter::LevelFilter;
use ulid::Ulid;

use crate::{
  client::Client,
  commands::{ChangeFileCommand, CloseFileCommand, NewSessionCommand, OpenFileCommand},
  server::Server,
  types::Location,
};

#[derive(Args)]
struct Get {
  #[arg(long, default_value_t = Server::DEFAULT_PORT)]
  port: u16,

  session_id: Option<Ulid>,
}

impl Get {
  async fn run(self) -> Result<(), AnyhowError> {
    Client::new(self.port)?
      .get(self.session_id)
      .await?
      .sessions
      .to_json_str()?
      .println()
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
  async fn run(self) -> Result<(), AnyhowError> {
    Client::new(self.port)?
      .new_session(&self.command)
      .await?
      .session_id
      .println()
      .ok()
  }
}

#[derive(Args)]
struct File {
  #[arg(long, default_value_t = Server::DEFAULT_PORT)]
  port: u16,

  #[command(subcommand)]
  command: FileCommand,
}

impl File {
  async fn run(self) -> Result<(), AnyhowError> {
    let client = Client::new(self.port)?;

    match self.command {
      FileCommand::Open(open_command) => client.open_file(&open_command).await?.ok(),
      FileCommand::Change(change_command) => client.change_file(change_command).await?.ok(),
      FileCommand::Close(close_command) => client.close_file(&close_command).await?.ok(),
    }
  }
}

#[derive(Subcommand)]
enum FileCommand {
  Open(OpenFileCommand),
  Change(ChangeFileCommand),
  Close(CloseFileCommand),
}

#[derive(Args)]
struct Notifications {
  #[arg(long, default_value_t = Server::DEFAULT_PORT)]
  port: u16,

  #[arg(long)]
  session_id: Option<Ulid>,
}

impl Notifications {
  async fn run(self) -> Result<(), AnyhowError> {
    let client = Client::new(self.port)?;
    let mut notifications = client.notifications(self.session_id).await?;

    while let Some(notification_res) = notifications.next().await {
      notification_res?.to_json_str()?.println();
    }

    ().ok()
  }
}

#[derive(Args)]
struct Serve {
  #[arg(long, default_value_t = Server::DEFAULT_PORT)]
  port: u16,
}

impl Serve {
  async fn run(self) -> Result<(), AnyhowError> {
    Server::serve(self.port).await
  }
}

#[derive(Subcommand)]
enum InfoViewCommand {
  GetPlainGoals(Location),
}

#[derive(Args)]
struct InfoView {
  #[arg(long, default_value_t = Server::DEFAULT_PORT)]
  port: u16,

  #[arg(long)]
  session_id: Option<Ulid>,

  #[command(subcommand)]
  command: InfoViewCommand,
}

impl InfoView {
  async fn run(self) -> Result<(), AnyhowError> {
    match self.command {
      InfoViewCommand::GetPlainGoals(command) => Client::new(self.port)?
        .get_plain_goals(self.session_id, command)
        .await?
        .to_json_str()?
        .println()
        .ok(),
    }
  }
}

#[derive(Args)]
struct Status {
  #[arg(long, default_value_t = Server::DEFAULT_PORT)]
  port: u16,
}

impl Status {
  async fn run(self) -> Result<(), AnyhowError> {
    Client::new(self.port)?.status().await?.to_json_str()?.println().ok()
  }
}

#[derive(Subcommand)]
enum Command {
  Get(Get),
  New(New),
  File(File),
  Notifications(Notifications),
  Serve(Serve),
  InfoView(InfoView),
  Status(Status),
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

  pub async fn run(self) -> Result<(), AnyhowError> {
    self.init_tracing();

    match self.command {
      Command::Get(get) => get.run().await,
      Command::New(new) => new.run().await,
      Command::File(open) => open.run().await,
      Command::Notifications(notifications) => notifications.run().await,
      Command::Serve(serve) => serve.run().await,
      Command::InfoView(info_view) => info_view.run().await,
      Command::Status(status) => status.run().await,
    }
  }
}
