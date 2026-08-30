use std::fmt::{Debug, Formatter, Result as FmtResult};

use crate::data_item::DataItem;

/// Content of an array, `CBOR` major type 4.
///
/// An array holds an ordered sequence of data items and remembers whether it
/// was encoded with a definite length or with the indefinite length form
/// terminated by a break. The items themselves are reached through
/// [`items`](Array::items) and its mutable and owning counterparts.
///
/// # Example
/// ```rust
/// use cbor_next::{Array, DataItem};
///
/// let mut content = Array::from_items([1, 2]);
/// content.extend([3]);
/// assert_eq!(content.items().len(), 3);
/// assert_eq!(content.items()[2], DataItem::from(3));
/// assert!(!content.is_indefinite());
/// ```
#[derive(Default, PartialEq, Eq, Clone, Hash)]
pub struct Array {
    is_indefinite: bool,
    items: Vec<DataItem>,
}

impl Array {
    /// Create an empty definite length array.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a definite length array from a sequence of items.
    #[must_use]
    pub fn from_items<I, T>(items: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<DataItem>,
    {
        Self {
            is_indefinite: false,
            items: items.into_iter().map(Into::into).collect(),
        }
    }

    /// Return the same content with the definite or indefinite encoding
    /// selected.
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::Array;
    ///
    /// let content = Array::from_items([1, 2]).with_indefinite(true);
    /// assert!(content.is_indefinite());
    /// ```
    #[must_use]
    pub fn with_indefinite(mut self, indefinite: bool) -> Self {
        self.is_indefinite = indefinite;
        self
    }

    /// Select the definite or indefinite encoding in place.
    pub fn set_indefinite(&mut self, indefinite: bool) {
        self.is_indefinite = indefinite;
    }

    /// Whether the array uses the indefinite length encoding.
    #[must_use]
    pub fn is_indefinite(&self) -> bool {
        self.is_indefinite
    }

    /// The items of the array.
    #[must_use]
    pub fn items(&self) -> &[DataItem] {
        &self.items
    }

    /// The items of the array, mutably.
    #[must_use]
    pub fn items_mut(&mut self) -> &mut Vec<DataItem> {
        &mut self.items
    }

    /// Take ownership of the items of the array.
    #[must_use]
    pub fn into_items(self) -> Vec<DataItem> {
        self.items
    }
}

impl<T> From<Vec<T>> for Array
where
    T: Into<DataItem>,
{
    fn from(value: Vec<T>) -> Self {
        Self::from_items(value)
    }
}

impl<T, const N: usize> From<[T; N]> for Array
where
    T: Into<DataItem>,
{
    fn from(value: [T; N]) -> Self {
        Self::from_items(value)
    }
}

impl<T> FromIterator<T> for Array
where
    T: Into<DataItem>,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::from_items(iter)
    }
}

impl<T> Extend<T> for Array
where
    T: Into<DataItem>,
{
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.items.extend(iter.into_iter().map(Into::into));
    }
}

impl Debug for Array {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if self.is_indefinite {
            f.write_str("[_ ")?;
        } else {
            f.write_str("[")?;
        }
        for (index, item) in self.items.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{item:?}")?;
        }
        f.write_str("]")
    }
}
