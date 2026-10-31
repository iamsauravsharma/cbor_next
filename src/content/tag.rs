use std::fmt::{Debug, Formatter, Result as FmtResult, Write as _};

use crate::data_item::DataItem;
use crate::decode::{Decoder, DuplicateKeyMode};
use crate::tags::{
    BASE64, BASE64URL, BIGFLOAT, DATE_TIME_STRING, DECIMAL_FRACTION, EMBEDDED_ITEM,
    EMBEDDED_SEQUENCE, EPOCH_TIME, MIME_MESSAGE, NEGATIVE_BIGNUM, UNSIGNED_BIGNUM, URI,
};

/// A tag, major type 6: a number and the item it applies to.
///
/// # Example
/// ```rust
/// use cbor_next::{DataItem, Tag};
///
/// let content = Tag::new(1, 1_363_896_240);
/// assert_eq!(content.number(), 1);
/// assert_eq!(content.content(), &DataItem::from(1_363_896_240));
/// ```
#[derive(PartialEq, Eq, Clone, Hash)]
pub struct Tag {
    number: u64,
    content: Box<DataItem>,
}

impl Tag {
    /// Create a tag.
    #[must_use]
    pub fn new(number: u64, content: impl Into<DataItem>) -> Self {
        Self {
            number,
            content: Box::new(content.into()),
        }
    }

    /// The tag number.
    #[must_use]
    pub fn number(&self) -> u64 {
        self.number
    }

    /// Replace the tag number.
    pub fn set_number(&mut self, number: u64) {
        self.number = number;
    }

    /// The tagged item.
    #[must_use]
    pub fn content(&self) -> &DataItem {
        &self.content
    }

    /// The tagged item, mutably.
    #[must_use]
    pub fn content_mut(&mut self) -> &mut DataItem {
        &mut self.content
    }

    /// Take the tagged item.
    #[must_use]
    pub fn into_content(self) -> DataItem {
        *self.content
    }

    /// Whether the content has the type the tag requires (RFC 8949 section
    /// 5.3.2).
    ///
    /// Checks the tags in [`tags`](crate::tags); others accept anything.
    /// Values are not checked, except that tags 24 and 63 need well formed
    /// `CBOR`.
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::{DataItem, Tag, tags};
    ///
    /// assert!(Tag::new(tags::EPOCH_TIME, 1_363_896_240).is_valid());
    /// assert!(!Tag::new(tags::EPOCH_TIME, "2013-03-21T20:04:00Z").is_valid());
    /// assert!(Tag::new(tags::EMBEDDED_ITEM, DataItem::bytes([0x01])).is_valid());
    /// assert!(!Tag::new(tags::EMBEDDED_ITEM, DataItem::bytes([0x18])).is_valid());
    /// assert!(Tag::new(1_000, "anything").is_valid());
    /// ```
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let content = self.content();
        match self.number {
            DATE_TIME_STRING | URI | BASE64URL | BASE64 | MIME_MESSAGE => content.is_text(),
            EPOCH_TIME => content.is_integer() || content.is_floating(),
            UNSIGNED_BIGNUM | NEGATIVE_BIGNUM => content.is_byte(),
            DECIMAL_FRACTION | BIGFLOAT => content.as_array().is_some_and(|array| {
                matches!(
                    array.items(),
                    [exponent, mantissa] if exponent.is_integer() && is_integer_or_bignum(mantissa)
                )
            }),
            EMBEDDED_ITEM => {
                content
                    .as_byte()
                    .is_some_and(|bytes| well_formed_decoder().decode(&bytes.to_bytes()).is_ok())
            }
            EMBEDDED_SEQUENCE => {
                content.as_byte().is_some_and(|bytes| {
                    well_formed_decoder()
                        .sequence(&bytes.to_bytes())
                        .all(|item| item.is_ok())
                })
            }
            _ => true,
        }
    }
}

/// Whether an item is an integer or a valid bignum.
fn is_integer_or_bignum(item: &DataItem) -> bool {
    item.is_integer()
        || item.as_tag().is_some_and(|tag| {
            (tag.number() == UNSIGNED_BIGNUM || tag.number() == NEGATIVE_BIGNUM) && tag.is_valid()
        })
}

/// A decoder that only checks well-formedness (RFC 8949 section 3.4.5.1).
fn well_formed_decoder() -> Decoder {
    Decoder::new().duplicate_keys(DuplicateKeyMode::Last)
}

impl<T> From<(u64, T)> for Tag
where
    T: Into<DataItem>,
{
    fn from((number, content): (u64, T)) -> Self {
        Self::new(number, content)
    }
}

/// Bignums print as their integer, other tags as `number(content)`.
impl Debug for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if let (UNSIGNED_BIGNUM | NEGATIVE_BIGNUM, DataItem::Byte(bytes)) =
            (self.number, self.content())
        {
            let negative = self.number == NEGATIVE_BIGNUM;
            if negative {
                f.write_char('-')?;
            }
            return f.write_str(&bignum_to_decimal(&bytes.to_bytes(), negative));
        }
        write!(f, "{}({:?})", self.number, self.content)
    }
}

/// Decimal digits of a big endian magnitude, plus one if `plus_one`
/// (a negative bignum `n` means `-1 - n`).
fn bignum_to_decimal(bytes: &[u8], plus_one: bool) -> String {
    // base 256, least significant first
    let mut digits_256 = bytes.iter().rev().copied().collect::<Vec<_>>();
    if plus_one {
        let mut carry = true;
        for digit in &mut digits_256 {
            (*digit, carry) = digit.overflowing_add(1);
            if !carry {
                break;
            }
        }
        if carry {
            digits_256.push(1);
        }
    }
    let mut decimal = Vec::new();
    loop {
        while digits_256.last() == Some(&0) {
            digits_256.pop();
        }
        if digits_256.is_empty() {
            break;
        }
        // divide by ten; the remainder is the next digit
        let mut remainder = 0_u16;
        for digit in digits_256.iter_mut().rev() {
            let value = (remainder << 8) | u16::from(*digit);
            *digit = u8::try_from(value / 10).expect("a quotient below 256");
            remainder = value % 10;
        }
        decimal.push(b'0' + u8::try_from(remainder).expect("a remainder below ten"));
    }
    if decimal.is_empty() {
        return "0".into();
    }
    decimal.reverse();
    String::from_utf8(decimal).expect("ascii digits")
}
