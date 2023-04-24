use std::{path::Path, process::Command, io::Result as IoResult};
use crate::config::CONFIG;

pub(crate) fn add_and_commit_to_index(path: &Path, message: &str) -> IoResult<()> {
    let current_dir = std::env::current_dir()?;
    std::env::set_current_dir(&CONFIG.index.path)?;
    assert!(Command::new("git")
        .args(["add", &format!("{}", path.display())])
        .status()?
        .success());
    assert!(Command::new("git")
        .args(["commit", "-m", message])
        .status()?
        .success());
    std::env::set_current_dir(current_dir)
}

pub(crate) fn init_index() -> IoResult<()> {
    let current_dir = std::env::current_dir()?;
    std::env::set_current_dir(&CONFIG.index.path)?;
    assert!(Command::new("git")
        .args(["init"])
        .status()?
        .success());
    std::env::set_current_dir(current_dir)
}