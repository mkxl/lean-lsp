use std::path::{Path, PathBuf};

use anyhow::Error;
use ulid::Ulid;

use crate::{process::Process, utils::Utils};

#[allow(dead_code)]
pub struct Session {
  key: Ulid,
  lean_filepath: PathBuf,
  process: Process,
  project_dirpath: PathBuf,
}

impl Session {
  pub fn new(
    lean_filepath: PathBuf,
    project_dirpath: Option<PathBuf>,
    lean_server_log_dirpath: Option<&Path>,
  ) -> Result<Self, Error> {
    let key = Ulid::new();
    let project_dirpath = Self::project_dirpath(&lean_filepath, project_dirpath)?;
    let process = Self::process(&project_dirpath, lean_server_log_dirpath)?;
    let session = Self {
      key,
      lean_filepath,
      process,
      project_dirpath,
    };

    session.ok()
  }

  fn get_project_dirpath(lean_filepath: &Path) -> Result<PathBuf, Error> {
    for ancestor_path in lean_filepath.ancestors() {
      let mut lakefile_filepath = ancestor_path.with_file_name("lakefile.lean");

      if lakefile_filepath.is_file() {
        lakefile_filepath.pop();

        return lakefile_filepath.ok();
      }
    }

    anyhow::bail!("unable to get project dirpath: no manifest file found in ancestor dirpaths");
  }

  fn project_dirpath(lean_filepath: &Path, project_dirpath: Option<PathBuf>) -> Result<PathBuf, Error> {
    if let Some(project_dirpath) = project_dirpath {
      project_dirpath.ok()
    } else {
      Self::get_project_dirpath(lean_filepath)
    }
  }

  fn process(project_dirpath: &Path, lean_server_log_dirpath: Option<&Path>) -> Result<Process, Error> {
    let env: &[(&str, &Path)] = if let Some(lean_server_log_dirpath) = lean_server_log_dirpath {
      &[("LEAN_SERVER_LOG_DIR", lean_server_log_dirpath)]
    } else {
      &[]
    };
    let process = Process::new("lake", &["serve"], env, project_dirpath.some())?;

    process.ok()
  }

  pub async fn run(&mut self) -> Result<(), Error> {
    self.process.run().await
  }
}
