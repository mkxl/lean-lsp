use std::io::Error as IoError;

use camino::Utf8PathBuf;
use clap::{Args, Subcommand};
use derive_more::{Constructor, From};
use mkutils::{FromChain, PointU16, Request, Screen, TypeAssoc, Utils};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::{
  lean_server_process::LeanServerProcess,
  responses::{GetPlainGoalsResponse, HoverFileResponse},
  session::Session,
  types::{AppError, Location, SessionInfo, Utf8},
};

#[derive(Args, Debug, Deserialize, Serialize, TypeAssoc)]
#[type_assoc(impl_trait = Request, Response = Result<(), AppError>, Serialized = Command)]
pub struct ChangeFileCommand {
  #[arg(long)]
  pub session_id: Option<Ulid>,

  #[arg(long = "input")]
  pub input_filepath: Utf8PathBuf,

  pub filepath: Utf8PathBuf,
}

#[derive(Args, Constructor, Debug, Deserialize, Serialize, TypeAssoc)]
#[type_assoc(impl_trait = Request, Response = Result<(), AppError>, Serialized = Command)]
pub struct CloseFileCommand {
  #[arg(long)]
  pub session_id: Option<Ulid>,

  pub filepath: Utf8PathBuf,
}

#[derive(Args, Constructor, Debug, Deserialize, Serialize, TypeAssoc)]
#[type_assoc(impl_trait = Request, Response = Result<HoverFileResponse, AppError>, Serialized = Command)]
pub struct HoverFileCommand {
  #[arg(long)]
  pub session_id: Option<Ulid>,

  #[command(flatten)]
  pub location: Location<Utf8>,
}

#[derive(Args, Constructor, Debug, Deserialize, Serialize, TypeAssoc)]
#[type_assoc(impl_trait = Request, Response = Result<(), AppError>, Serialized = Command)]
pub struct OpenFileCommand {
  #[arg(long)]
  pub session_id: Option<Ulid>,

  pub filepath: Utf8PathBuf,
}

#[derive(Debug, Deserialize, From, Serialize, Subcommand)]
pub enum FileCommand {
  Change(ChangeFileCommand),
  Close(CloseFileCommand),
  Hover(HoverFileCommand),
  Open(OpenFileCommand),
}

#[derive(Args, Debug, Deserialize, Serialize, TypeAssoc)]
#[type_assoc(impl_trait = Request, Response = Result<SessionInfo, AppError>, Serialized = Command)]
pub struct GetCommand {
  pub session_id: Option<Ulid>,
}

#[derive(Args, Constructor, Debug, Deserialize, Serialize, TypeAssoc)]
#[type_assoc(impl_trait = Request, Response = Result<GetPlainGoalsResponse, AppError>, Serialized = Command)]
pub struct GetPlainGoalsCommand {
  #[arg(long)]
  pub session_id: Option<Ulid>,

  #[command(flatten)]
  pub location: Location<Utf8>,
}

#[derive(Debug, Deserialize, From, Serialize, Subcommand)]
pub enum InfoViewCommand {
  GetPlainGoals(GetPlainGoalsCommand),
}

#[derive(Args, Debug, Deserialize, Serialize, TypeAssoc)]
#[type_assoc(impl_trait = Request, Response = Result<(), AppError>, Serialized = Command)]
pub struct KillCommand {
  pub session_id: Option<Ulid>,
}

#[derive(Args, Debug, Deserialize, Serialize, TypeAssoc)]
#[type_assoc(impl_trait = Request, Response = Vec<SessionInfo>, Serialized = Command)]
pub struct ListCommand;

#[derive(Args, Constructor, Debug, Deserialize, Serialize, TypeAssoc)]
#[type_assoc(impl_trait = Request, Response = Result<SessionInfo, AppError>, Serialized = Command)]
pub struct NewSessionCommand {
  #[arg(long = "lake", default_value = LeanServerProcess::DEFAULT_LAKE_FILEPATH_STR)]
  pub lake_filepath: Utf8PathBuf,

  #[arg(long = "log-dir", env = LeanServerProcess::LOG_DIRPATH_ENV_NAME)]
  pub lean_server_log_dirpath: Option<Utf8PathBuf>,

  #[arg(long)]
  pub enrich_utf16_positions: bool,

  #[arg(value_name = "path", default_value = Session::DEFAULT_PATH_STR, value_parser = Self::absolute_path)]
  pub absolute_path: Utf8PathBuf,
}

impl NewSessionCommand {
  fn absolute_path(path_str: &str) -> Result<Utf8PathBuf, IoError> {
    path_str.absolute_utf8()?.into_owned().ok()
  }
}

#[derive(Args, Debug, Deserialize, Serialize, TypeAssoc)]
#[type_assoc(impl_trait = Request, Serialized = Command)]
pub struct NotificationsCommand {
  #[arg(long)]
  pub session_id: Option<Ulid>,

  #[arg(long = "method")]
  pub methods: Vec<String>,
}

// NOTE-524ed3: use [NewSessionCommand::Response] as [RebuildCommand::Response]
// because we send a new initialize request to `lake` after building
#[derive(Args, Debug, Deserialize, Serialize, TypeAssoc)]
#[type_assoc(impl_trait = Request, Response = <NewSessionCommand as Request>::Response, Serialized = Command)]
pub struct RebuildCommand {
  #[arg(long)]
  pub session_id: Option<Ulid>,
}

#[derive(Args, Debug, Deserialize, Serialize, TypeAssoc)]
#[type_assoc(impl_trait = Request, Serialized = Command)]
pub struct TuiCommand {
  #[arg(long)]
  pub session_id: Option<Ulid>,

  #[arg(skip = Screen::size().unwrap())]
  pub size: PointU16,
}

#[derive(Debug, Deserialize, From, FromChain, Serialize, Subcommand)]
#[from(ChangeFileCommand, FileCommand)]
#[from(CloseFileCommand, FileCommand)]
#[from(HoverFileCommand, FileCommand)]
#[from(OpenFileCommand, FileCommand)]
#[from(GetPlainGoalsCommand, InfoViewCommand)]
pub enum Command {
  #[command(subcommand)]
  File(FileCommand),

  Get(GetCommand),

  #[command(subcommand)]
  InfoView(InfoViewCommand),

  Kill(KillCommand),

  List(ListCommand),

  New(NewSessionCommand),

  Notifications(NotificationsCommand),

  Rebuild(RebuildCommand),

  Serve,

  Tui(TuiCommand),
}
