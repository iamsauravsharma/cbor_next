use std::string::FromUtf8Error;

use crate::data_item::DataItem;

/// A decoding error.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// Input ended inside an item.
    UnexpectedEnd {
        /// Bytes still needed.
        missing: u64,
    },
    /// Input ended before the break of an indefinite length item.
    IncompleteIndefinite,
    /// A text string chunk is not valid UTF-8.
    InvalidUtf8(FromUtf8Error),
    /// Additional information 28 to 30, or 31 outside major types 2 to 5.
    InvalidAdditionalInfo {
        /// Major type of the head.
        major_type: u8,
        /// Additional information found.
        additional: u8,
    },
    /// A string chunk's major type differs from its string's.
    InvalidChunkMajorType {
        /// Major type of the string.
        expected: u8,
        /// Major type of the chunk.
        found: u8,
    },
    /// A simple value of 20 to 31, or below 32 in the two byte form.
    InvalidSimpleValue(u8),
    /// A break outside an indefinite length item.
    UnexpectedBreak,
    /// A map repeats a key; see [`Decoder::duplicate_keys`].
    ///
    /// [`Decoder::duplicate_keys`]: crate::Decoder::duplicate_keys
    DuplicateMapKey(Box<DataItem>),
    /// An indefinite length item, disallowed by
    /// [`Decoder::allow_indefinite`].
    ///
    /// [`Decoder::allow_indefinite`]: crate::Decoder::allow_indefinite
    IndefiniteNotAllowed,
    /// Not deterministic, as [`Decoder::require_deterministic`] asks.
    ///
    /// [`Decoder::require_deterministic`]: crate::Decoder::require_deterministic
    NotDeterministic,
    /// Nesting is deeper than the limit.
    RecursionLimit {
        /// The limit.
        limit: usize,
    },
    /// More items than [`Decoder::max_items`] allows.
    ///
    /// [`Decoder::max_items`]: crate::Decoder::max_items
    ItemLimit {
        /// The limit.
        limit: usize,
    },
    /// Bytes remain after the item.
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
            Self::ItemLimit { limit } => {
                write!(f, "number of data items exceeds maximum of {limit}")
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
