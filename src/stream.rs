use anyhow::Error as AnyhowError;
use mkutils::Utils;
use poem::web::websocket::{Message as PoemMessage, WebSocketStream};
use ulid::Ulid;

use crate::{session::Session, session_set::SessionSet};

pub struct Stream;

impl Stream {
  pub async fn on_web_socket_upgrade(
    session_set: SessionSet,
    mut web_socket_stream: WebSocketStream,
  ) -> Result<(), AnyhowError> {
    loop {
      let PoemMessage::Text(message) = web_socket_stream.next_item_async().await?? else { continue };
      let mut message_json = message.to_json()?;
      let session_id = message_json.take_json("session_id")?;
      let response_json = match message_json.take_json::<String>("type")?.as_str() {
        "new_session" => session_set
          .new_session(
            message_json.take_json("lean_path")?,
            message_json.take_json("lean_server_log_dirpath")?,
          )
          .await?
          .id()
          .to_json_object("session_id"),
        "get_sessions" => session_set
          .get_sessions()
          .await?
          .iter()
          .map_collect::<Ulid, Vec<_>>(Session::id)
          .to_json_object("session_ids"),
        "get_session" => session_set
          .get_session(session_id)
          .await?
          .id()
          .to_json_object("session_id"),
        "initialize" => session_set
          .get_session(session_id)
          .await?
          .initialize()
          .await?
          .with("complete")
          .to_json_object("initialize"),
        "open_file" => session_set
          .get_session(session_id)
          .await?
          .open_file(message_json.take_json("filepath")?)
          .await?
          .with("complete")
          .to_json_object("open_file"),
        "close_file" => session_set
          .get_session(session_id)
          .await?
          .close_file(message_json.take_json("filepath")?)
          .await?
          .with("complete")
          .to_json_object("close_file"),
        "get_plain_goals" => session_set
          .get_session(session_id)
          .await?
          .get_plain_goals(message_json.take_json("location")?)
          .await?
          .to_json()?,
        "get_status" => session_set.get_session(session_id).await?.status().await?.to_json()?,
        _ => serde_json::json!({"error": "unknown type"}),
      };

      response_json
        .to_json_str()?
        .poem_text_message()
        .send_to(&mut web_socket_stream)
        .await?;
    }
  }
}
