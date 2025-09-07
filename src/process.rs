use std::{
  borrow::Borrow,
  ffi::OsStr,
  path::Path,
  process::{ExitStatus, Stdio},
};

use anyhow::{Context, Error};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};

use crate::utils::Utils;

pub struct Process {
  child: Child,
  stdin: ChildStdin,
  stdout: ChildStdout,
  stderr: ChildStderr,
}

impl Process {
  pub fn new<Cmd: AsRef<OsStr>, Args: IntoIterator, Env: IntoIterator, K: AsRef<OsStr>, V: AsRef<OsStr>>(
    cmd: Cmd,
    args: Args,
    env: Env,
    current_dirpath: Option<&Path>,
  ) -> Result<Self, Error>
  where
    Args::Item: AsRef<OsStr>,
    Env::Item: Borrow<(K, V)>,
  {
    let mut command = Command::new(cmd);

    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    for arg in args {
      command.arg(arg);
    }

    for env_entry in env {
      let (env_var_name, env_var_val) = env_entry.borrow();

      command.env(env_var_name, env_var_val);
    }

    if let Some(current_dirpath) = current_dirpath {
      command.current_dir(current_dirpath);
    }

    let mut child = command.spawn()?;
    let stdin = Self::take_stdio(&mut child.stdin)?;
    let stdout = Self::take_stdio(&mut child.stdout)?;
    let stderr = Self::take_stdio(&mut child.stderr)?;
    let process = Self {
      child,
      stdin,
      stdout,
      stderr,
    };

    process.ok()
  }

  fn take_stdio<T>(stdio: &mut Option<T>) -> Result<T, Error> {
    stdio.take().context("unable to set up stdio for process")
  }

  pub fn stdin_mut(&mut self) -> &mut ChildStdin {
    &mut self.stdin
  }

  pub fn stdout_mut(&mut self) -> &mut ChildStdout {
    &mut self.stdout
  }

  pub fn stderr_mut(&mut self) -> &mut ChildStderr {
    &mut self.stderr
  }

  pub async fn run(&mut self) -> Result<ExitStatus, Error> {
    self.child.wait().await?.ok()
  }
}
