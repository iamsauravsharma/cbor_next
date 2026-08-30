use std::fmt::{Debug, Formatter, Result as FmtResult};

use crate::data_item::DataItem;

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
