use std::{marker::Unpin, str::Utf8Error};

use anyhow::Error;
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
}

impl<T: ?Sized> Utils for T {}
