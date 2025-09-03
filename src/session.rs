use std::path::{Path, PathBuf};

use anyhow::Error;
use ulid::Ulid;

use crate::{process::Process, utils::Utils};

#[allow(dead_code)]
pub struct Session {
  key: Ulid,
  process: Process,
  project_dirpath: PathBuf,
}

impl Session {
  pub const LEAN_SERVER_LOG_DIRPATH_ENV_NAME: &'static str = "LEAN_SERVER_LOG_DIR";

  const MANIFEST_FILE_NAMES: &[&'static str] = &["lakefile.lean", "lakefile.toml"];

  pub fn new(lean_path: &Path, lean_server_log_dirpath: Option<&Path>) -> Result<Self, Error> {
    let key = Ulid::new();
    let project_dirpath = Self::project_dirpath(lean_path)?;
    let process = Self::process(&project_dirpath, lean_server_log_dirpath)?;
    let session = Self {
      key,
      process,
      project_dirpath,
    };

    tracing::info!(message = "new session", project_dirpath = %session.project_dirpath.display());

    session.ok()
  }

  fn project_dirpath(lean_path: &Path) -> Result<PathBuf, Error> {
    for ancestor_path in lean_path.ancestors() {
      for manifest_file_name in Self::MANIFEST_FILE_NAMES {
        let mut manifest_filepath = ancestor_path.with_file_name(manifest_file_name);

        if manifest_filepath.is_file() {
          manifest_filepath.pop();

          return manifest_filepath.ok();
        }
      }
    }

    anyhow::bail!("unable to get project dirpath: no manifest file found in ancestor dirpaths");
  }

  fn process(project_dirpath: &Path, lean_server_log_dirpath: Option<&Path>) -> Result<Process, Error> {
    let lean_server_log_dirpath = lean_server_log_dirpath.map(Path::as_os_str).unwrap_or_default();
    let process = Process::new(
      "lake",
      ["serve"],
      [(Self::LEAN_SERVER_LOG_DIRPATH_ENV_NAME, lean_server_log_dirpath)],
      project_dirpath.some(),
    )?;

    process.ok()
  }

  pub async fn run(&mut self) -> Result<(), Error> {
    self.process.run().await
  }
}
