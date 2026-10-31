use std::fmt::{Display, Formatter, Result as FmtResult};

/// A serde conversion error.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// A custom message from serde or an implementation.
    Message(String),
    /// Data the other side cannot hold, such as an unexpected tag, a bignum
    /// past 128 bits or a repeated map key.
    Unsupported(String),
    /// The bytes failed to decode.
    Cbor(crate::Error),
}

pub(super) fn unsupported(what: impl Display) -> Error {
    Error::Unsupported(what.to_string())
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Message(message) => f.write_str(message),
            Self::Unsupported(what) => write!(f, "{what} is not supported"),
            Self::Cbor(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Cbor(error) => Some(error),
            _ => None,
        }
    }
}

impl From<crate::Error> for Error {
    fn from(value: crate::Error) -> Self {
        Self::Cbor(value)
    }
}

impl ::serde::ser::Error for Error {
    fn custom<T: Display>(message: T) -> Self {
        Self::Message(message.to_string())
    }
}

impl ::serde::de::Error for Error {
    fn custom<T: Display>(message: T) -> Self {
        Self::Message(message.to_string())
    }
}
