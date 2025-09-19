use std::{
  borrow::Borrow,
  ffi::OsStr,
  fmt::{Debug, Display},
  io::Error as IoError,
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
  fn absolute(&self) -> Result<PathBuf, IoError>
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

  fn into_string(self) -> Result<String, Error>
  where
    Self: Is<PathBuf> + Sized,
  {
    match self.get().into_os_string().into_string() {
      Ok(string) => string.ok(),
      Err(os_string) => anyhow::bail!("{os_string:?} is not valid unicode"),
    }
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
    self.get().map(Y::from)
  }

  fn map_as_ref<'a, Y: ?Sized, X: 'a + AsRef<Y> + ?Sized>(self) -> Option<&'a Y>
  where
    Self: Is<Option<&'a X>> + Sized,
  {
    self.get().map(X::as_ref)
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

  fn to_uri(&self) -> Result<String, Error>
  where
    Self: AsRef<Path>,
  {
    "file://".cat(self.absolute()?.display()).ok()
  }

  fn unit(&self) {}
}

impl<T: ?Sized> Utils for T {}
