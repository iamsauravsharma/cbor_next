/// Enum representing error for a crate
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("CBOR bytes cannot be empty")]
    Empty,
    #[error(transparent)]
    FromUtf8(#[from] std::string::FromUtf8Error),
    #[error("incomplete indefinite length data")]
    IncompleteIndefinite,
    #[error("invalid simple value simple value cannot be between 24-32")]
    InvalidSimple,
    #[error(transparent)]
    FromInt(#[from] std::num::TryFromIntError),
    #[error("not well formed data")]
    NotWellFormed,
    #[error("break stop cannot be by itself")]
    LonelyBreakStop,
}
