use std::path::Path;

use anyhow::Error;

use crate::{process::Process, utils::Utils};

pub struct LeanServer {
  process: Process,
}

impl LeanServer {
  pub const LOG_DIRPATH_ENV_NAME: &'static str = "LEAN_SERVER_LOG_DIR";

  pub fn new(project_dirpath: &Path, log_dirpath: Option<&Path>) -> Result<Self, Error> {
    let process = Self::process(project_dirpath, log_dirpath)?;
    let lean_server = Self { process };

    lean_server.ok()
  }

  fn process(project_dirpath: &Path, log_dirpath: Option<&Path>) -> Result<Process, Error> {
    let log_dirpath = log_dirpath.map(Path::as_os_str).unwrap_or_default();
    let process = Process::new(
      "lake",
      ["serve"],
      [(Self::LOG_DIRPATH_ENV_NAME, log_dirpath)],
      project_dirpath.some(),
    )?;

    process.ok()
  }

  pub async fn run(&mut self) -> Result<(), Error> {
    self.process.run().await
  }
}
