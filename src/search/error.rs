use std::{
    error::Error,
    fmt::{Formatter, Display, Result as FMTResult}, num::ParseIntError,
};

#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum SearchResultError{
    EmptyVector,
    ParseU32(ParseIntError),
    MissingVersionElements
}

impl Error for SearchResultError{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ParseU32(i) => Some(i),
            _ => None
        }
    }
}
impl Display for SearchResultError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FMTResult {
        write!(f, "{}", match self {
            Self::EmptyVector => "no unyanked crates",
            Self::ParseU32(_) => "failed to parse number in version",
            Self::MissingVersionElements => "missing version elements"
        })
    }
}

impl From<ParseIntError> for SearchResultError {
    fn from(value: ParseIntError) -> Self {
        Self::ParseU32(value)
    }
}