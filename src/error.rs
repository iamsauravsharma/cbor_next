/// Enum representing error for a crate
#[derive(thiserror::Error, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Incomplete CBOR bytes
    #[error("Incomplete CBOR bytes")]
    Incomplete,
    /// Error generated when converted string from utf8 bytes
    #[error(transparent)]
    FromUtf8(#[from] std::string::FromUtf8Error),
    /// Incomplete indefinite length data
    #[error("incomplete indefinite length data")]
    IncompleteIndefinite,
    /// Invalid simple value
    #[error("invalid simple value simple value cannot be between 20-32")]
    InvalidSimple,
    /// Error converting to a required integer
    #[error(transparent)]
    FromInt(#[from] std::num::TryFromIntError),
    /// Not well formed data
    #[error("not well formed data")]
    NotWellFormed(String),
    /// Invalid break stop position
    #[error("break stop position is invalid")]
    InvalidBreakStop,
}
