use derive_more::From;
use poem_openapi::Object;
use uuid::Uuid;

// TODO: use macros to automatically construct from [SessionSetCommand]
#[derive(Object)]
pub struct NewSessionCommand {
  pub lean_path: String,
  pub lean_server_log_dirpath: Option<String>,
}

#[derive(From, Object)]
pub struct NewSessionResult {
  pub session_id: Uuid,
}

#[derive(From, Object)]
pub struct GetSessionsResult {
  pub session_ids: Vec<Uuid>,
}
