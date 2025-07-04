use std::num::TryFromIntError;
use std::string::FromUtf8Error;

/// Enum representing error for a crate
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// Incomplete CBOR bytes
    // #[error("Incomplete CBOR bytes")]
    Incomplete,
    /// Error generated when converted string from utf8 bytes
    FromUtf8(FromUtf8Error),
    /// Incomplete indefinite length data
    IncompleteIndefinite,
    /// Invalid simple value
    InvalidSimple,
    /// Error converting to a required integer
    FromInt(TryFromIntError),
    /// Not well formed data
    NotWellFormed(String),
    /// Invalid break stop position
    InvalidBreakStop,
}

impl From<FromUtf8Error> for Error {
    fn from(value: FromUtf8Error) -> Self {
        Self::FromUtf8(value)
    }
}

impl From<TryFromIntError> for Error {
    fn from(value: TryFromIntError) -> Self {
        Self::FromInt(value)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Incomplete => write!(f, "incomplete CBOR bytes"),
            Self::FromUtf8(internal_err) => internal_err.fmt(f),
            Self::IncompleteIndefinite => write!(f, "incomplete indefinite length data"),
            Self::InvalidSimple => {
                write!(
                    f,
                    "invalid simple value simple value cannot be between 20-32"
                )
            }
            Self::FromInt(internal_err) => internal_err.fmt(f),
            Self::NotWellFormed(internal_message) => {
                write!(f, "not well formed data : {internal_message}")
            }
            Self::InvalidBreakStop => write!(f, "break stop position is invalid"),
        }
    }
}

impl std::error::Error for Error {}
