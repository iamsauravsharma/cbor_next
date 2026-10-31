use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::hash::{Hash, Hasher};

use crate::data_item::DataItem;

/// An array, major type 4.
///
/// Remembers whether it uses the indefinite length form.
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
///
/// Equality ignores the length form.
#[derive(Default, Clone)]
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

    /// Create a definite length array from items.
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

    /// Set the length form, builder style.
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

    /// Set the length form.
    pub fn set_indefinite(&mut self, indefinite: bool) {
        self.is_indefinite = indefinite;
    }

    /// Whether the indefinite length form is used.
    #[must_use]
    pub fn is_indefinite(&self) -> bool {
        self.is_indefinite
    }

    /// The items.
    #[must_use]
    pub fn items(&self) -> &[DataItem] {
        &self.items
    }

    /// The items, mutably.
    #[must_use]
    pub fn items_mut(&mut self) -> &mut Vec<DataItem> {
        &mut self.items
    }

    /// Take the items.
    #[must_use]
    pub fn into_items(self) -> Vec<DataItem> {
        self.items
    }
}

impl PartialEq for Array {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items
    }
}

impl Eq for Array {}

impl Hash for Array {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.items.hash(state);
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
