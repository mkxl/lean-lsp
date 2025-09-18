use std::{
  borrow::Borrow,
  ffi::OsStr,
  fmt::{Debug, Display},
  marker::Unpin,
  path::{Path, PathBuf},
  str::Utf8Error,
};

use anyhow::{Context, Error};
use poem_openapi::payload::Json as PoemJson;
use serde::Serialize;
use serde_json::{Error as SerdeJsonError, Value as Json};
use tokio::{io::AsyncReadExt, sync::oneshot::Sender as OneshotSender, task::JoinHandle};

use crate::is::Is;

pub trait Utils {
  fn absolute(&self) -> Result<PathBuf, Error>
  where
    Self: AsRef<Path>,
  {
    std::path::absolute(self)?.ok()
  }

  fn as_utf8(&self) -> Result<&str, Utf8Error>
  where
    Self: AsRef<[u8]>,
  {
    str::from_utf8(self.as_ref())
  }

  fn cat<T: Display>(&self, rhs: T) -> String
  where
    Self: Display,
  {
    std::format!("{self}{rhs}")
  }

  fn convert<T: From<Self>>(self) -> T
  where
    Self: Sized,
  {
    self.into()
  }

  fn file_name_ok(&self) -> Result<&OsStr, Error>
  where
    Self: AsRef<Path>,
  {
    self.as_ref().file_name().context("path has no file_name")
  }

  fn json_byte_str(&self) -> Result<Vec<u8>, SerdeJsonError>
  where
    Self: Serialize,
  {
    serde_json::to_vec(self)
  }

  fn json(&self) -> Result<Json, SerdeJsonError>
  where
    Self: Serialize,
  {
    serde_json::to_value(self)
  }

  fn log_error<T, C: Display, E: Debug + Display>(self, context: C) -> Self
  where
    Self: Borrow<Result<T, E>> + Sized,
  {
    if let Err(err) = self.borrow() {
      tracing::warn!(?err, "{context}: {err}");
    }

    self
  }

  fn map_into<X, Y: From<X>>(self) -> Option<Y>
  where
    Self: Is<Option<X>> + Sized,
  {
    self.take().map(Y::from)
  }

  fn ok<E>(self) -> Result<Self, E>
  where
    Self: Sized,
  {
    Ok(self)
  }

  fn pair<T>(self, rhs: T) -> (Self, T)
  where
    Self: Sized,
  {
    (self, rhs)
  }

  fn poem_json(self) -> PoemJson<Self>
  where
    Self: Sized,
  {
    PoemJson(self)
  }

  async fn read_string(&mut self) -> Result<String, Error>
  where
    Self: AsyncReadExt + Unpin,
  {
    let mut string = String::new();

    self.read_to_string(&mut string).await?;

    string.ok()
  }

  fn send_to_oneshot(self, sender: OneshotSender<Self>) -> Result<(), Error>
  where
    Self: Sized,
  {
    // NOTE: drop error variant which wraps [Self] and may not implement [StdError]
    sender
      .send(self)
      .ok()
      .context("unable to send value over oneshot channel")
  }

  fn some(self) -> Option<Self>
  where
    Self: Sized,
  {
    Some(self)
  }

  fn spawn_task(self) -> JoinHandle<Self::Output>
  where
    Self: 'static + Future + Sized + Send,
    Self::Output: 'static + Send,
  {
    tokio::spawn(self)
  }

  // TODO-4eef0b: permit reverse search
  fn substr_interval(&self, query: &[u8]) -> Option<(usize, usize)>
  where
    Self: AsRef<[u8]>,
  {
    let bytes = self.as_ref();
    let predicate = |substr| substr == query;
    let query_len = query.len();
    let begin = bytes.windows(query_len).position(predicate)?;
    let end = begin + query_len;

    (begin, end).some()
  }

  fn to_str_ok(&self) -> Result<&str, Error>
  where
    Self: AsRef<Path>,
  {
    self.as_ref().to_str().context("path is not utf8")
  }

  fn to_uri(&self) -> Result<String, Error>
  where
    Self: AsRef<Path>,
  {
    "file://".cat(self.absolute()?.to_str_ok()?).ok()
  }

  fn unit(&self) {}
}

impl<T: ?Sized> Utils for T {}
