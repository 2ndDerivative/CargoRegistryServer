use std::{error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
    io::Error as IoError, num::{ParseIntError, TryFromIntError},
    string::FromUtf8Error,
};
use serde_json::error::Error as SerdeJsonError;

#[derive(Debug)]
pub(crate) enum PublishError{
    IoError(IoError),
    VersionAlreadyExists,
    BadIndexJson,
    SerializationFailed(SerdeJsonError),
    CrateExistsWithDifferentDashUnderscore,
}
impl Error for PublishError {}
impl Display for PublishError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Failed to publish: {}", match self {
            Self::IoError(e) => e.to_string(),
            Self::BadIndexJson => "bad index json".to_string(),
            Self::VersionAlreadyExists => "version already exists".to_string(),
            Self::SerializationFailed(e) => format!("serialization of index crate failed: {e}"),
            Self::CrateExistsWithDifferentDashUnderscore => "crate exists with different dash/underscore name".to_string()
        })
    }
}

impl From<IoError> for PublishError {
    fn from(value: IoError) -> Self {
        Self::IoError(value)
    }
}

impl From<SerdeJsonError> for PublishError {
    fn from(value: SerdeJsonError) -> Self {
        Self::SerializationFailed(value)
    }
}

impl From<ParseIntError> for ReadStreamError {
    fn from(value: ParseIntError) -> Self {
        Self::NonNumericContentLength(value)
    }
}



#[derive(Debug)]
pub(crate) enum ReadStreamError {
    ConnectionClosed(IoError),
    BadHTTPJson(SerdeJsonError),
    NonNumericContentLength(ParseIntError),
    InvalidUTF8Error(FromUtf8Error),
    PayloadTooLarge,
}
impl Error for ReadStreamError {}
impl Display for ReadStreamError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Failed to get package from stream: {}",
            match self {
                Self::BadHTTPJson(j) => format!("no valid package json: {j}"),
                Self::ConnectionClosed(e) => e.to_string(),
                Self::NonNumericContentLength(n) => format!("\"Content-Length\" is not a number: {n}"),
                Self::InvalidUTF8Error(i) => format!("{i}"),
                Self::PayloadTooLarge => format!("request body too large for server platform! Max: {}", usize::MAX),
            }
        )
    }
}

impl From<SerdeJsonError> for ReadStreamError {
    fn from(value: SerdeJsonError) -> Self {
        ReadStreamError::BadHTTPJson(value)
    }
}

impl From<FromUtf8Error> for ReadStreamError {
    fn from(value: FromUtf8Error) -> Self {
        ReadStreamError::InvalidUTF8Error(value)
    }
}

impl From<IoError> for ReadStreamError {
    fn from(value: IoError) -> Self {
        Self::ConnectionClosed(value)
    }
}

impl From<TryFromIntError> for ReadStreamError {
    fn from(_value: TryFromIntError) -> Self {
        Self::PayloadTooLarge
    }
}
