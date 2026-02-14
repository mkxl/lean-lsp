use std::{io::Error as IoError, time::Duration};

use anyhow::Error as AnyhowError;
use mkutils::{Event, Socket, Utils};
use tokio::net::{UnixListener, UnixStream, unix::SocketAddr};

use crate::{
  commands::{Command, GetCommand, KillCommand, ListCommand},
  session_map::SessionMap,
  tui::TuiMap,
  types::AppError,
};

#[derive(Default)]
pub struct Server {
  session_map: SessionMap,
  tui_map: TuiMap,
  kill_event: Event,
}

impl Server {
  pub const SOCKET_FILEPATH_STR: &str = "/tmp/lean-lsp.sock";

  const ON_SERVE_COMMAND_ERROR_MESSAGE: &str = "lean-lsp server is already running";
  const DURATION_RENDER_PERIOD: Duration = Duration::from_millis(20);
  const DURATION_KEEP_ALIVE_PERIOD: Duration = Duration::from_secs(5);

  fn on_serve_command() -> Result<(), AppError> {
    anyhow::anyhow!(Self::ON_SERVE_COMMAND_ERROR_MESSAGE).err()?
  }

  #[tracing::instrument(skip_all, err)]
  async fn on_unix_stream(&mut self, pair_res: Result<(UnixStream, SocketAddr), IoError>) -> Result<(), AnyhowError> {
    let (unix_stream, _socket_addr) = pair_res?;
    let mut socket = unix_stream.convert::<Socket>();
    let command = socket.recv::<Command>().await.into_option().check_next()??;

    match command {
      Command::File(file_command) => self.session_map.on_file_command(socket, file_command).await,
      Command::Get(get_command) => {
        self
          .session_map
          .on_get_command(&get_command)
          .respond_to::<GetCommand>(socket)
          .await
      }
      Command::InfoView(info_view_command) => self.session_map.on_info_view_command(socket, &info_view_command).await,
      Command::Kill(kill_command) => {
        self
          .session_map
          .on_kill_command(&self.kill_event, &kill_command)
          .await
          .respond_to::<KillCommand>(socket)
          .await
      }
      Command::List(_list_command) => {
        self
          .session_map
          .on_list_command()
          .respond_to::<ListCommand>(socket)
          .await
      }
      Command::New(new_session_command) => {
        self
          .session_map
          .on_new_session_command(socket, new_session_command)
          .await
      }
      Command::Notifications(notifications_command) => {
        self.session_map.on_notifications_command(socket, notifications_command)
      }
      Command::Rebuild(rebuild_command) => self.session_map.on_rebuild_command(socket, &rebuild_command).await,
      Command::Serve => Self::on_serve_command().send_to(socket).await,
      Command::Tui(tui_command) => self.tui_map.on_tui_command(&self.session_map, socket, &tui_command),
    }
  }

  async fn serve_impl(mut self) -> Result<(), AnyhowError> {
    Self::SOCKET_FILEPATH_STR.remove_file().unit();

    let listener = UnixListener::bind(Self::SOCKET_FILEPATH_STR)?;
    let mut render_interval = Self::DURATION_RENDER_PERIOD.into_interval();
    let mut keep_alive_interval = Self::DURATION_KEEP_ALIVE_PERIOD.into_interval();

    loop {
      tokio::select! {
        pair_res = listener.accept() => self.on_unix_stream(pair_res).await,
        session_message = self.session_map.next_message() => session_message.session.on_message(session_message.message).await,
        tui_event = self.tui_map.next_event() => self.tui_map.on_tui_event(tui_event).await,
        _instant = render_interval.tick() => self.tui_map.render().await,
        _instant = keep_alive_interval.tick() => self.session_map.send_keep_alive().await,
        () = self.kill_event.wait() => return ().ok()
      }
      .log_if_error()
      .unit();
    }
  }

  #[tracing::instrument(err)]
  pub async fn serve() -> Result<(), AnyhowError> {
    Self::default().serve_impl().await
  }
}
