/// Enum representing error for a crate
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Empty CBOR bytes
    #[error("CBOR bytes cannot be empty")]
    Empty,
    /// Error generated when converted string from utf8 bytes
    #[error(transparent)]
    FromUtf8(#[from] std::string::FromUtf8Error),
    /// Incomplete indefinite length data
    #[error("incomplete indefinite length data")]
    IncompleteIndefinite,
    /// Invalid simple value
    #[error("invalid simple value simple value cannot be between 24-32")]
    InvalidSimple,
    /// Error converting to a required integer
    #[error(transparent)]
    FromInt(#[from] std::num::TryFromIntError),
    /// Not well formed data
    #[error("not well formed data")]
    NotWellFormed,
    /// Break stop without using indefinite length
    #[error("break stop cannot be by itself")]
    LonelyBreakStop,
}
