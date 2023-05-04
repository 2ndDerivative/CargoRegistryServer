use std::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult}
};

#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum AddOwnerError {
    NoSuchUser,
    MultipleUsers,
    SqlError(rusqlite::Error)
}
impl Error for AddOwnerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        if let AddOwnerError::SqlError(i) = self {
            return Some(i)
        }
        None
    }
}
impl Display for AddOwnerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            AddOwnerError::MultipleUsers => write!(f, "Multiple users with that name exist"),
            AddOwnerError::NoSuchUser => write!(f, "There is no user with that name"),
            AddOwnerError::SqlError(i) => write!(f, "Database access failed {i}")
        }    
    }
}

impl From<rusqlite::Error> for AddOwnerError {
    fn from(value: rusqlite::Error) -> Self {
        AddOwnerError::SqlError(value)
    }
}