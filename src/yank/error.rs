use std::{
    error::Error,
    fmt::{Formatter, Display, Result as FMTResult},
    io::Error as IoError,
};

#[derive(Debug)]
pub(crate) enum YankError {
    ConnectionClosed(IoError),
}
impl Error for YankError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ConnectionClosed(i) => Some(i)
        }
    }
}
impl Display for YankError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FMTResult {
        write!(f, "failed to (un)yank")
    }
}

impl From<IoError> for YankError {
    fn from(value: IoError) -> Self {
        Self::ConnectionClosed(value)
    }
}
