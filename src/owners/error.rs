use std::{
    error::Error,
    fmt::{Display, Formatter, Result as FMTResult}, 
    num::ParseIntError,
    io::Error as IoError, string::FromUtf8Error,
};

use serde_json::error::Error as JsonError;

#[derive(Debug)]
pub enum UnableToGetUsers {
    NoHeaderContentLength,
    ContentLengthNotANumber(ParseIntError),
    NoUTF8Json(FromUtf8Error),
    IoError(IoError),
    SerdeError(JsonError)
}
impl Error for UnableToGetUsers{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::NoHeaderContentLength => None,
            Self::ContentLengthNotANumber(p) => Some(p),
            Self::NoUTF8Json(u) => Some(u),
            Self::IoError(i) => Some(i),
            Self::SerdeError(s) => Some(s)
        }
    }
}
impl Display for UnableToGetUsers {
    fn fmt(&self, f: &mut Formatter<'_>) -> FMTResult {
        use UnableToGetUsers::{ContentLengthNotANumber, IoError, NoHeaderContentLength, NoUTF8Json};
        write!(f, "Unable to get users: {}", match self {
            NoHeaderContentLength => r#"No header "Content-Length""#,
            ContentLengthNotANumber(_) => "Content-Length not a number",
            NoUTF8Json(_) => "Json is not a valid string",
            IoError(_) => "IO Error during user readout",
            Self::SerdeError(_) => "Bad JSON"
        })
    }
}

impl From<ParseIntError> for UnableToGetUsers {
    fn from(value: ParseIntError) -> Self {
        UnableToGetUsers::ContentLengthNotANumber(value)
    }
}

impl From<FromUtf8Error> for UnableToGetUsers {
    fn from(value: FromUtf8Error) -> Self {
        UnableToGetUsers::NoUTF8Json(value)
    }
}

impl From<IoError> for UnableToGetUsers {
    fn from(value: IoError) -> Self {
        UnableToGetUsers::IoError(value)
    }
}

impl From<JsonError> for UnableToGetUsers {
    fn from(value: JsonError) -> Self {
        UnableToGetUsers::SerdeError(value)
    }
}