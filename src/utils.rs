use std::{
  ffi::OsStr,
  marker::Unpin,
  path::{Path, PathBuf},
  str::Utf8Error,
};

use anyhow::{Context, Error};
use serde::Serialize;
use serde_json::Error as SerdeJsonError;
use tokio::io::AsyncReadExt;

pub trait Utils {
  fn as_utf8(&self) -> Result<&str, Utf8Error>
  where
    Self: AsRef<[u8]>,
  {
    str::from_utf8(self.as_ref())
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

  async fn read_string(&mut self) -> Result<String, Error>
  where
    Self: AsyncReadExt + Unpin,
  {
    let mut string = String::new();

    self.read_to_string(&mut string).await?;

    string.ok()
  }

  fn some(self) -> Option<Self>
  where
    Self: Sized,
  {
    Some(self)
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

  fn json_string(&self) -> Result<String, SerdeJsonError>
  where
    Self: Serialize,
  {
    serde_json::to_string(self)
  }

  fn absolute(&self) -> Result<PathBuf, Error>
  where
    Self: AsRef<Path>,
  {
    std::path::absolute(self).context("path is not absolute")
  }

  fn file_name_ok(&self) -> Result<&OsStr, Error>
  where
    Self: AsRef<Path>,
  {
    self.as_ref().file_name().context("path has no file_name")
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
    format!("file://{}", self.absolute()?.to_str_ok()?).ok()
  }
}

impl<T: ?Sized> Utils for T {}
