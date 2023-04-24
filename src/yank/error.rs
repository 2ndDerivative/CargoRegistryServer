use std::{
    error::Error,
    fmt::{Formatter, Display, Result as FMTResult},
    io::Error as IoError,
};

#[derive(Debug, PartialEq)]
pub(crate) enum YankError {
    ConnectionClosed,
    InvalidPath(YankPathError)
}
impl Error for YankError {
}
impl Display for YankError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FMTResult {
        write!(f, "failed to (un)yank")
    }
}

impl From<YankPathError> for YankError {
    fn from(value: YankPathError) -> Self {
        Self::InvalidPath(value)
    }
}

impl From<IoError> for YankError {
    fn from(_value: IoError) -> Self {
        Self::ConnectionClosed
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum YankPathError {
    NoVersion,
    NoName,
    DotDotInPath,
    InvalidUTF8Error
}
impl Error for YankPathError {}
impl Display for YankPathError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FMTResult {
        match self {
            Self::NoVersion => write!(f, "path contains no version"),
            Self::NoName => write!(f, "path contains no crate name"),
            Self::DotDotInPath => write!(f, r#"".." in path"#),
            Self::InvalidUTF8Error => write!(f, "Invalid unicode in path"),
        }
    }
}