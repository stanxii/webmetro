use std::{
    error::Error,
    fmt::{
        Display,
        Formatter,
        Result as FmtResult
    },
    io::Error as IoError
};

use ebml::EbmlError;

#[derive(Debug)]
pub enum WebmetroError {
    EbmlError(EbmlError),
    IoError(IoError),
    Unknown(Box<Error>)
}

impl Display for WebmetroError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            &WebmetroError::EbmlError(ref err) => err.fmt(f),
            &WebmetroError::IoError(ref err) => err.fmt(f),
            &WebmetroError::Unknown(ref err) => err.fmt(f),
        }
    }
}
impl Error for WebmetroError {
    fn description(&self) -> &str {
        match self {
            &WebmetroError::EbmlError(ref err) => err.description(),
            &WebmetroError::IoError(ref err) => err.description(),
            &WebmetroError::Unknown(ref err) => err.description(),
        }
    }
}

impl From<EbmlError> for WebmetroError {
    fn from(err: EbmlError) -> WebmetroError {
        WebmetroError::EbmlError(err)
    }
}

impl From<IoError> for WebmetroError {
    fn from(err: IoError) -> WebmetroError {
        WebmetroError::IoError(err)
    }
}

impl From<Box<Error>> for WebmetroError {
    fn from(err: Box<Error>) -> WebmetroError {
        WebmetroError::Unknown(err)
    }
}
