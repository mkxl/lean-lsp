use std::{
  ffi::OsStr,
  io::{PipeReader, PipeWriter, Write},
  path::Path,
};

use anyhow::Error;
use tokio::process::{Child, Command};

use crate::utils::Utils;

#[allow(dead_code)]
pub struct Process {
  child: Child,
  stdin_writer: PipeWriter,
  stdout_reader: PipeReader,
  stderr_reader: PipeReader,
}

impl Process {
  pub fn new<S1: AsRef<OsStr>, S2: AsRef<OsStr>, S3: AsRef<OsStr>, S4: AsRef<OsStr>>(
    cmd: S1,
    args: impl IntoIterator<Item = S2>,
    env: impl IntoIterator<Item = (S3, S4)>,
    current_dirpath: Option<&Path>,
  ) -> Result<Self, Error> {
    let mut command = Command::new(cmd);

    for arg in args {
      command.arg(arg);
    }

    for (env_var_name, env_var_val) in env {
      command.env(env_var_name, env_var_val);
    }

    if let Some(current_dirpath) = current_dirpath {
      command.current_dir(current_dirpath);
    }

    let (stdin_reader, stdin_writer) = std::io::pipe()?;
    let (stdout_reader, stdout_writer) = std::io::pipe()?;
    let (stderr_reader, stderr_writer) = std::io::pipe()?;

    command.stdin(stdin_reader);
    command.stdout(stdout_writer);
    command.stderr(stderr_writer);

    let child = command.spawn()?;
    let process = Self {
      child,
      stdin_writer,
      stdout_reader,
      stderr_reader,
    };

    process.ok()
  }

  #[allow(dead_code)]
  pub fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
    self.stdin_writer.write_all(bytes)?;
    self.stdin_writer.flush()?;

    ().ok()
  }

  pub async fn run(&mut self) -> Result<(), Error> {
    self.child.wait().await?.unit().ok()
  }
}
