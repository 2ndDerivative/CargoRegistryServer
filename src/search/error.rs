use std::{
    error::Error,
    fmt::{Formatter, Display, Result as FMTResult}, num::ParseIntError,
};

#[derive(Debug)]
pub enum SearchResultError{
    EmptyVector,
    ParseU32,
    MissingVersionElements
}
impl Error for SearchResultError{}
impl Display for SearchResultError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FMTResult {
        write!(f, "{}", match self {
            Self::EmptyVector => "no unyanked crates",
            Self::ParseU32 => "failed to parse number in version",
            Self::MissingVersionElements => "missing version elements"
        })
    }
}

impl From<ParseIntError> for SearchResultError {
    fn from(_value: ParseIntError) -> Self {
        Self::ParseU32
    }
}