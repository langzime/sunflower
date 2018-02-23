use std::result::Result;
use std::io::Write;
use std::error::Error;
use std::fmt::{self, Formatter};
use std::convert::From;
use std::io::Error as IoError;
use httparse::Error as HttpError;
use std::num::ParseIntError;
use serde_json::Error as SerdeJsonError;

pub type MioResult<T> = Result<T, MioError>;

#[derive( Debug)]
pub enum MioError {
    IoError(IoError),
    HttpParseError(HttpError),
    ParseIntError(ParseIntError),
    SerdeJsonError(SerdeJsonError),
    Error(String),
}

impl From<IoError> for MioError {
    fn from(error: IoError) -> Self {
        MioError::IoError(error)
    }
}

impl From<HttpError> for MioError {
    fn from(error: HttpError) -> Self {
        MioError::HttpParseError(error)
    }
}

impl From<ParseIntError> for MioError {
    fn from(error: ParseIntError) -> Self {
        MioError::ParseIntError(error)
    }
}

impl From<SerdeJsonError> for MioError {
    fn from(error: SerdeJsonError) -> Self {
        MioError::SerdeJsonError(error)
    }
}

impl Error for MioError {
    fn description(&self) -> &str {
        match *self {
            MioError::IoError(ref err) => err.description(),
            MioError::HttpParseError(ref err) => err.description(),
            MioError::ParseIntError(ref err) => err.description(),
            MioError::SerdeJsonError(ref err) => err.description(),
            MioError::Error(ref err) => err,

        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            MioError::IoError(ref err) => Some(err),
            MioError::HttpParseError(ref err) => Some(err),
            MioError::ParseIntError(ref err) => Some(err),
            MioError::SerdeJsonError(ref err) => Some(err),

            MioError::Error(_) => None,
        }
    }
}

impl fmt::Display for MioError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            MioError::IoError(ref err) => write!(f, "MioError error: {}", err),
            MioError::HttpParseError(ref err) => write!(f, "MioError error: {}", err),
            MioError::ParseIntError(ref err) => write!(f, "MioError error: {}", err),
            MioError::SerdeJsonError(ref err) => write!(f, "MioError error: {}", err),
            MioError::Error(ref inner) => inner.fmt(f),
        }
    }
}