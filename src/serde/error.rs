use std::fmt::{Display, Formatter, Result as FmtResult};

/// Error returned when converting between Rust values and data items.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// A message raised by serde or by a `Serialize` or `Deserialize`
    /// implementation.
    Message(String),
    /// Data which the other side cannot represent.
    ///
    /// It is rejected as it is, never changed to fit: a tag which nothing
    /// asked for, `undefined` or a simple value in place of another type, a
    /// bignum wider than 128 bits and a map which uses a key twice are all
    /// refused.
    Unsupported(String),
    /// Bytes which are not a well formed data item.
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
