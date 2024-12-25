//! A generic error type.
use std::{
    error,
    fmt::{self, Debug, Display, Formatter},
};

/// A generic error type that can be fully sent between threads.
pub struct GenericError(String);

impl Debug for GenericError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("Error({})", self.0))
    }
}

impl Display for GenericError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("Error: {}", self.0))
    }
}

impl From<&dyn ToString> for GenericError {
    fn from(value: &dyn ToString) -> Self {
        Self(value.to_string())
    }
}

impl From<souvlaki::Error> for GenericError {
    fn from(value: souvlaki::Error) -> Self {
        Self(format!("{value:?}"))
    }
}

impl error::Error for GenericError {}
