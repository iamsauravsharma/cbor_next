use std::fmt::{Debug, Formatter, Result as FmtResult};

use crate::data_item::DataItem;
use crate::decode::{Decoder, DuplicateKeyMode};
use crate::tags::{
    BASE64, BASE64URL, BIGFLOAT, DATE_TIME_STRING, DECIMAL_FRACTION, EMBEDDED_ITEM,
    EMBEDDED_SEQUENCE, EPOCH_TIME, MIME_MESSAGE, NEGATIVE_BIGNUM, UNSIGNED_BIGNUM, URI,
};

/// Content of a tagged item, `CBOR` major type 6.
///
/// A tag pairs an unsigned integer, which carries the semantic meaning
/// registered for it, with the single data item it applies to.
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
    /// Create a tag from its number and the item it applies to.
    #[must_use]
    pub fn new(number: u64, content: impl Into<DataItem>) -> Self {
        Self {
            number,
            content: Box::new(content.into()),
        }
    }

    /// The number of the tag.
    #[must_use]
    pub fn number(&self) -> u64 {
        self.number
    }

    /// Replace the number of the tag.
    pub fn set_number(&mut self, number: u64) {
        self.number = number;
    }

    /// The item the tag applies to.
    #[must_use]
    pub fn content(&self) -> &DataItem {
        &self.content
    }

    /// The item the tag applies to, mutably.
    #[must_use]
    pub fn content_mut(&mut self) -> &mut DataItem {
        &mut self.content
    }

    /// Take ownership of the item the tag applies to.
    #[must_use]
    pub fn into_content(self) -> DataItem {
        *self.content
    }

    /// Whether the content is of the type the tag number requires.
    ///
    /// This is the first kind of tag validity of RFC 8949 section 5.3.2,
    /// checked for the tags registered by RFC 8949 and RFC 8742, which are
    /// named in [`tags`](crate::tags). A tag of any other number is valid
    /// with any content. The value of the content is not checked, so tag 0
    /// around text which is not a date is valid here, while tag 24 and tag 63
    /// do require their byte string to hold well formed `CBOR`.
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

/// Whether an item is an integer of major type 0 or 1 or a valid bignum.
fn is_integer_or_bignum(item: &DataItem) -> bool {
    item.is_integer()
        || item.as_tag().is_some_and(|tag| {
            (tag.number() == UNSIGNED_BIGNUM || tag.number() == NEGATIVE_BIGNUM) && tag.is_valid()
        })
}

/// A decoder which checks that bytes are well formed and nothing more, as
/// RFC 8949 section 3.4.5.1 asks of tag 24.
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

impl Debug for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}({:?})", self.number, self.content)
    }
}
