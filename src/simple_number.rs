use std::ops::Deref;

use crate::error::Error;

/// struct representing simple number which only allow number between 0-19 and
/// 32 -255
///
/// # Example
/// ```rust
/// use cbor_next::SimpleNumber;
///
/// assert!(SimpleNumber::try_from(10).is_ok());
/// assert!(SimpleNumber::try_from(100).is_ok());
/// assert!(SimpleNumber::try_from(255).is_ok());
/// assert!(SimpleNumber::try_from(24).is_err());
/// assert!(SimpleNumber::try_from(29).is_err());
/// ```
#[derive(PartialEq, Debug, Hash, Clone)]
pub struct SimpleNumber(u8);

impl Deref for SimpleNumber {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl SimpleNumber {
    /// Get internal number value
    #[must_use]
    pub fn val(&self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for SimpleNumber {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0..=19 | 32..=u8::MAX => Ok(Self(value)),
            _ => Err(Error::InvalidSimple),
        }
    }
}
