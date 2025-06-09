/// Enum representing error for a crate
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("CBOR bytes cannot be empty")]
    Empty,
    #[error("Invalid CBOR : {0}")]
    Invalid(String),
}
