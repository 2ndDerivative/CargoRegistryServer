use std::{
    error::Error,
    io::Error as IoError,
    fmt::{Display, Formatter, Result as FmtResult},
    path::PathBuf
};

use serde_json::error::Error as SerdeError;

#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum WalkIndexError {
    ParseJson(SerdeError, PathBuf, usize),
    IoError(IoError, PathBuf)
}
impl Error for WalkIndexError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ParseJson(p, _, _) => Some(p),
            Self::IoError(i, _) => Some(i)
        }
    }
}

impl Display for WalkIndexError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::ParseJson(_, p, l) => write!(f, "Failed to parse JSON in file {} at line {l}", p.display()),
            Self::IoError(_, p) => write!(f, "Failed to read out file {}", p.display()),
        }
    }
}