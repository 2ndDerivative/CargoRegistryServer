use std::{
    path::Path, 
    process::{Command, Stdio}, 
    io::Result as IoResult,
    env::{current_dir, set_current_dir},
};
use crate::config::CONFIG;

pub(crate) fn add_and_commit_to_index<P: AsRef<Path>>(relative_path: &P, message: &str) -> IoResult<()> {
    let current_dir = current_dir()?;
    set_current_dir(&CONFIG.index.path)?;
    Command::new("git")
        .arg("add")
        .arg(relative_path.as_ref())
        .stdout(Stdio::null())
        .status()?;
    Command::new("git")
        .args(["commit", "-m", message, "--no-gpg-sign"])
        .stdout(Stdio::null())
        .status()?;
    set_current_dir(current_dir)
}

pub(crate) fn init_index() -> IoResult<()> {
    let current_dir = current_dir()?;
    set_current_dir(&CONFIG.index.path)?;
    Command::new("git")
        .args(["init"])
        .stdout(Stdio::null())
        .status()?;
    set_current_dir(current_dir)
}