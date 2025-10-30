use std::path::PathBuf;

use clap::Args;
use derive_more::Constructor;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

#[derive(Args, Constructor, Deserialize, Object, Serialize)]
pub struct Location {
  pub filepath: PathBuf,

  #[arg(long)]
  pub line: usize,

  #[arg(long)]
  pub character: usize,
}
