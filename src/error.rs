use std::string::FromUtf8Error;

use crate::data_item::DataItem;

/// Error returned when a byte sequence cannot be decoded as `CBOR`.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// Input ended in the middle of a data item.
    ///
    /// `missing` is how many further bytes the item being decoded asked for.
    UnexpectedEnd {
        /// Number of bytes the truncated data item is short of.
        missing: u64,
    },
    /// Input ended before the break which closes an indefinite length item.
    IncompleteIndefinite,
    /// A text string chunk is not valid UTF-8.
    InvalidUtf8(FromUtf8Error),
    /// The additional information of a head is not allowed for its major type.
    ///
    /// The values 28 to 30 are reserved for every major type, and 31, the
    /// indefinite length marker, is only allowed for major types 2 to 5.
    InvalidAdditionalInfo {
        /// Major type of the head which carried the additional information.
        major_type: u8,
        /// Additional information found in the head.
        additional: u8,
    },
    /// A chunk of an indefinite length string has the wrong major type.
    ///
    /// Every chunk of an indefinite length byte or text string must repeat the
    /// major type of the string it belongs to.
    InvalidChunkMajorType {
        /// Major type of the enclosing indefinite length string.
        expected: u8,
        /// Major type found on the chunk.
        found: u8,
    },
    /// A simple value uses a number which RFC 8949 assigns or reserves.
    InvalidSimpleValue(u8),
    /// A break appeared where no indefinite length item was open.
    UnexpectedBreak,
    /// The same key appeared more than once in a map.
    ///
    /// Decoding with [`Decoder::allow_duplicate_keys`] accepts such a map
    /// instead, keeping the last value of the key.
    ///
    /// [`Decoder::allow_duplicate_keys`]: crate::Decoder::allow_duplicate_keys
    DuplicateMapKey(Box<DataItem>),
    /// An indefinite length item appeared while indefinite lengths were
    /// rejected by [`Decoder::allow_indefinite`].
    ///
    /// [`Decoder::allow_indefinite`]: crate::Decoder::allow_indefinite
    IndefiniteNotAllowed,
    /// A well formed item was decoded but it is not in the deterministic form
    /// demanded by [`Decoder::require_deterministic`].
    ///
    /// [`Decoder::require_deterministic`]: crate::Decoder::require_deterministic
    NotDeterministic,
    /// Nesting of a data item went past the configured maximum depth.
    RecursionLimit {
        /// Maximum nesting depth which was applied.
        limit: usize,
    },
    /// Bytes remain after a complete data item was decoded.
    TrailingBytes {
        /// Number of bytes left over.
        count: usize,
    },
}

impl From<FromUtf8Error> for Error {
    fn from(value: FromUtf8Error) -> Self {
        Self::InvalidUtf8(value)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedEnd { missing } => {
                write!(f, "incomplete CBOR bytes: {missing} byte missing")
            }
            Self::IncompleteIndefinite => write!(f, "incomplete indefinite length data"),
            Self::InvalidUtf8(internal_err) => internal_err.fmt(f),
            Self::InvalidAdditionalInfo {
                major_type,
                additional,
            } => {
                write!(
                    f,
                    "invalid additional information {additional} for major type {major_type}"
                )
            }
            Self::InvalidChunkMajorType { expected, found } => {
                write!(
                    f,
                    "contains invalid major type {found} for indefinite major type {expected}"
                )
            }
            Self::InvalidSimpleValue(value) => {
                write!(
                    f,
                    "invalid simple value {value}: simple value cannot be between 20 and 31"
                )
            }
            Self::UnexpectedBreak => write!(f, "break stop position is invalid"),
            Self::DuplicateMapKey(key) => {
                write!(f, "same map key {key} is repeated multiple times")
            }
            Self::IndefiniteNotAllowed => {
                write!(f, "indefinite length data item is not allowed")
            }
            Self::NotDeterministic => {
                write!(f, "data item is not in deterministic form")
            }
            Self::RecursionLimit { limit } => {
                write!(f, "nesting depth of data item exceeds maximum of {limit}")
            }
            Self::TrailingBytes { count } => {
                write!(
                    f,
                    "{count} extra bytes remain after a complete CBOR data item"
                )
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidUtf8(internal_err) => Some(internal_err),
            _ => None,
        }
    }
}
