use std::fmt::{Debug, Formatter, Result as FmtResult, Write as _};

use crate::data_item::DataItem;
#[cfg(feature = "validation")]
use crate::decode::Decoder;
use crate::tags::{NEGATIVE_BIGNUM, UNSIGNED_BIGNUM};
use crate::validate;

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
    /// Checks the tags in [`tags`](crate::tags), including the shape of
    /// structured content such as IP prefixes; others accept anything. Values
    /// are not checked, except that tags 24 and 63 need well formed `CBOR`.
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::{DataItem, Tag, tags};
    ///
    /// assert!(Tag::new(tags::EPOCH_TIME, 1_363_896_240).is_valid());
    /// assert!(Tag::new(tags::IPV4, DataItem::bytes([192, 0, 2, 1])).is_valid());
    /// assert!(!Tag::new(tags::IPV4, DataItem::bytes([192, 0, 2])).is_valid());
    /// assert!(!Tag::new(tags::EPOCH_TIME, "2013-03-21T20:04:00Z").is_valid());
    /// assert!(Tag::new(tags::EMBEDDED_ITEM, DataItem::bytes([0x01])).is_valid());
    /// assert!(!Tag::new(tags::EMBEDDED_ITEM, DataItem::bytes([0x18])).is_valid());
    /// assert!(Tag::new(1_000, "anything").is_valid());
    /// ```
    #[must_use]
    pub fn is_valid(&self) -> bool {
        validate::has_valid_type(self)
    }

    /// Whether the content's value is right too: an RFC 3339 date and time
    /// (tag 0) or `full-date` (1004), a URI reference (32), base64url without
    /// or base64 with padding (33, 34), valid `CBOR` for the default
    /// [`Decoder`] (24, 63), and an IP prefix with no bits past its length
    /// (52, 54). A MIME message (36) is only checked to be text.
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::{Tag, tags};
    ///
    /// assert!(Tag::new(tags::DATE_TIME_STRING, "2013-03-21T20:04:00Z").has_valid_value());
    /// assert!(!Tag::new(tags::DATE_TIME_STRING, "2013-02-30T20:04:00Z").has_valid_value());
    /// assert!(!Tag::new(tags::URI, "a b").has_valid_value());
    /// ```
    #[must_use]
    #[cfg(feature = "validation")]
    #[cfg_attr(docsrs, doc(cfg(feature = "validation")))]
    pub fn has_valid_value(&self) -> bool {
        validate::has_valid_value(self, &Decoder::new())
    }
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
