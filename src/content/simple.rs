use std::fmt::{Debug, Formatter, Result as FmtResult};

use crate::error::Error;

/// A simple value (major type 7) without its own variant: 0 to 19 or 32 to 255.
///
/// 20 to 23 are `false`, `true`, `null` and `undefined`; 24 to 31 are reserved.
///
/// # Example
/// ```rust
/// use cbor_next::Simple;
///
/// assert_eq!(Simple::try_from(10).unwrap().value(), 10);
/// assert!(Simple::try_from(255).is_ok());
/// assert!(Simple::try_from(24).is_err());
/// assert!(Simple::try_from(29).is_err());
/// ```
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct Simple(u8);

impl Simple {
    /// The number.
    #[must_use]
    pub fn value(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for Simple {
    type Error = Error;

    /// Create a simple value.
    ///
    /// # Errors
    /// If the number is between 20 and 31
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0..=19 | 32..=u8::MAX => Ok(Self(value)),
            _ => Err(Error::InvalidSimpleValue(value)),
        }
    }
}

impl From<Simple> for u8 {
    fn from(value: Simple) -> Self {
        value.0
    }
}

impl Debug for Simple {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "simple({})", self.0)
    }
}
