use std::io::{Error as IoError, Write};

use anyhow::Error as AnyhowError;
use clap::Parser;
use crossterm::event::EventStream as CrosstermEventStream;
use futures::{SinkExt, StreamExt};
use mkutils::{Output, Screen, Socket, Tracing, Utils};
use tracing_subscriber::filter::LevelFilter;

use crate::{
  commands::{
    Command, FileCommand, GetCommand, InfoViewCommand, KillCommand, ListCommand, NewSessionCommand,
    NotificationsCommand, TuiCommand,
  },
  notification::Notification,
  server::Server,
};

#[derive(Parser)]
pub struct CliArgs {
  #[arg(long = "log-level", default_value_t = Tracing::DEFAULT_LEVEL_FILTER, env = Self::LOG_LEVEL_ENV_NAME)]
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
  const LOG_LEVEL_ENV_NAME: &str = "LOG_LEVEL";
  const TUI_WITH_MOUSE_CAPTURE: bool = true;

  fn init_tracing(&self) {
    Tracing::default()
      .with_level_filter(self.tracing_level_filter)
      .with_json_enabled(!self.tracing_json_disabled)
      .with_tokio_console_port(self.tracing_tokio_console_port)
      .with_tokio_console_enabled(self.tracing_tokio_console_enabled)
      .with_stderr_lock_writer()
      .init();
  }

  async fn socket() -> Result<Socket, IoError> {
    Socket::connect(Server::SOCKET_FILEPATH_STR.as_ref()).await
  }

  async fn file(file_command: FileCommand) -> Result<(), AnyhowError> {
    let mut socket = Self::socket().await?;

    match file_command {
      FileCommand::Change(change_file_command) => socket.request(change_file_command).await??,
      FileCommand::Close(close_file_command) => socket.request(close_file_command).await??,
      FileCommand::Hover(hover_file_command) => socket.request(hover_file_command).await??.to_json_str()?.println(),
      FileCommand::Open(open_file_command) => socket.request(open_file_command).await??,
    }

    ().ok()
  }

  async fn get(get_command: GetCommand) -> Result<(), AnyhowError> {
    Self::socket()
      .await?
      .request(get_command)
      .await??
      .to_json_str()?
      .println()
      .ok()
  }

  async fn info_view(info_view_command: InfoViewCommand) -> Result<(), AnyhowError> {
    let mut socket = Self::socket().await?;

    match info_view_command {
      InfoViewCommand::GetPlainGoals(get_plain_goals_command) => {
        socket.request(get_plain_goals_command).await??.to_json_str()?.println();
      }
    }

    ().ok()
  }

  async fn kill(kill_command: KillCommand) -> Result<(), AnyhowError> {
    Self::socket().await?.request(kill_command).await??.ok()
  }

  async fn list(list_command: ListCommand) -> Result<(), AnyhowError> {
    Self::socket()
      .await?
      .request(list_command)
      .await?
      .to_json_str()?
      .println()
      .ok()
  }

  async fn new_session(new_session_command: NewSessionCommand) -> Result<(), AnyhowError> {
    Self::socket()
      .await?
      .request(new_session_command)
      .await??
      .to_json_str()?
      .println()
      .ok()
  }

  async fn notifications(notifications_command: NotificationsCommand) -> Result<(), AnyhowError> {
    let mut socket = Self::socket().await?;

    socket.serialize(notifications_command).await?;

    while let Some(notification_res) = socket.recv::<Notification>().await.into_option() {
      notification_res?.to_json_str()?.println();
    }

    ().ok()
  }

  async fn tui(tui_command: TuiCommand) -> Output<(), AnyhowError> {
    let mut socket = Self::socket().await?;
    let mut crossterm_event_stream = CrosstermEventStream::new();
    let mut screen = Screen::config()
      .with_mouse_capture(Self::TUI_WITH_MOUSE_CAPTURE)
      .build()?;

    socket.serialize(tui_command).await?;

    loop {
      tokio::select! {
        event_res_opt = crossterm_event_stream.next() => socket.send(event_res_opt.check_next()??).await?,
        byte_str_output = socket.recv::<Vec<u8>>() => screen.writer_mut().write_all_then(&byte_str_output?)?.flush()?,
      }
    }
  }

  pub async fn run(self) -> Result<(), AnyhowError> {
    self.init_tracing();

    match self.command {
      Command::File(file_command) => Self::file(file_command).await,
      Command::Get(get_command) => Self::get(get_command).await,
      Command::InfoView(info_view_command) => Self::info_view(info_view_command).await,
      Command::Kill(kill_command) => Self::kill(kill_command).await,
      Command::List(list_command) => Self::list(list_command).await,
      Command::New(new_session_command) => Self::new_session(new_session_command).await,
      Command::Notifications(notifications_command) => Self::notifications(notifications_command).await,
      Command::Serve => Server::serve().await,
      Command::Tui(tui_command) => Self::tui(tui_command).await.into_end(),
    }
  }
}
