use std::{
    path::Path, 
    process::Command, 
    io::Result as IoResult,
    env::{current_dir, set_current_dir},
};
use crate::config::CONFIG;

pub(crate) fn add_and_commit_to_index(path: &Path, message: &str) -> IoResult<()> {
    let current_dir = current_dir()?;
    set_current_dir(&CONFIG.index.path)?;
    Command::new("git")
        .args(["add", &format!("{}", path.display())])
        .status()?;
    Command::new("git")
        .args(["commit", "-m", message])
        .status()?;
    set_current_dir(current_dir)
}

pub(crate) fn init_index() -> IoResult<()> {
    let current_dir = current_dir()?;
    set_current_dir(&CONFIG.index.path)?;
    Command::new("git")
        .args(["init"])
        .status()?;
    set_current_dir(current_dir)
}