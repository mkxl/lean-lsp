use std::path::{Path, PathBuf};

use anyhow::Error;
use ulid::Ulid;

use crate::{lean_server::LeanServer, utils::Utils};

pub struct Session {
  key: Ulid,
  lean_server: LeanServer,
  project_dirpath: PathBuf,
}

impl Session {
  pub fn new(lean_path: &Path, lean_server_log_dirpath: Option<&Path>) -> Result<Self, Error> {
    let key = Ulid::new();
    let project_dirpath = Self::project_dirpath(lean_path)?;
    let lean_server = LeanServer::new(&project_dirpath, lean_server_log_dirpath)?;
    let session = Self {
      key,
      lean_server,
      project_dirpath,
    };

    tracing::info!(message = "new session", key = %session.key, project_dirpath = %session.project_dirpath.display());

    session.ok()
  }

  fn project_dirpath(lean_path: &Path) -> Result<PathBuf, Error> {
    for ancestor_path in lean_path.ancestors() {
      let mut manifest_filepath = ancestor_path.with_file_name("lake-manifest.json");

      if manifest_filepath.is_file() {
        manifest_filepath.pop();

        return manifest_filepath.ok();
      }
    }

    anyhow::bail!("unable to get project dirpath: no manifest file found in ancestor dirpaths");
  }

  pub async fn run(&mut self) -> Result<(), Error> {
    self.lean_server.run().await
  }
}
