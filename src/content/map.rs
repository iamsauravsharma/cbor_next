use std::fmt::{Debug, Formatter, Result as FmtResult};

use indexmap::IndexMap;

use crate::data_item::DataItem;

/// A map, major type 5.
///
/// Keeps entry order (an [`IndexMap`]) and its length form, so it re-encodes
/// the same.
///
/// # Example
/// ```rust
/// use cbor_next::{DataItem, Map};
///
/// let mut content = Map::new();
/// content.extend([("a", 1), ("b", 2)]);
/// assert_eq!(content.entries().len(), 2);
/// assert_eq!(
///     content.entries().get(&DataItem::from("a")),
///     Some(&DataItem::from(1))
/// );
/// ```
///
/// Equality and hashing ignore entry order and length form.
#[derive(Default, Clone)]
pub struct Map {
    is_indefinite: bool,
    entries: IndexMap<DataItem, DataItem>,
}

impl Map {
    /// Create an empty definite length map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a definite length map from key value pairs.
    ///
    /// A repeated key keeps its first position and its last value.
    #[must_use]
    pub fn from_entries<I, K, V>(entries: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<DataItem>,
        V: Into<DataItem>,
    {
        Self {
            is_indefinite: false,
            entries: entries
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        }
    }

    /// Set the length form, builder style.
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::Map;
    ///
    /// let content = Map::from_entries([("a", 1)]).with_indefinite(true);
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

    /// The entries, in encoding order.
    #[must_use]
    pub fn entries(&self) -> &IndexMap<DataItem, DataItem> {
        &self.entries
    }

    /// The entries, mutably.
    #[must_use]
    pub fn entries_mut(&mut self) -> &mut IndexMap<DataItem, DataItem> {
        &mut self.entries
    }

    /// Take the entries.
    #[must_use]
    pub fn into_entries(self) -> IndexMap<DataItem, DataItem> {
        self.entries
    }
}

impl PartialEq for Map {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

impl Eq for Map {}

impl<K, V> From<IndexMap<K, V>> for Map
where
    K: Into<DataItem>,
    V: Into<DataItem>,
{
    fn from(value: IndexMap<K, V>) -> Self {
        Self::from_entries(value)
    }
}

impl<K, V> From<Vec<(K, V)>> for Map
where
    K: Into<DataItem>,
    V: Into<DataItem>,
{
    fn from(value: Vec<(K, V)>) -> Self {
        Self::from_entries(value)
    }
}

impl<K, V, const N: usize> From<[(K, V); N]> for Map
where
    K: Into<DataItem>,
    V: Into<DataItem>,
{
    fn from(value: [(K, V); N]) -> Self {
        Self::from_entries(value)
    }
}

impl<K, V> FromIterator<(K, V)> for Map
where
    K: Into<DataItem>,
    V: Into<DataItem>,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self::from_entries(iter)
    }
}

impl<K, V> Extend<(K, V)> for Map
where
    K: Into<DataItem>,
    V: Into<DataItem>,
{
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        self.entries.extend(
            iter.into_iter()
                .map(|(key, value)| (key.into(), value.into())),
        );
    }
}

impl Debug for Map {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if self.is_indefinite {
            f.write_str("{_ ")?;
        } else {
            f.write_str("{")?;
        }
        for (index, (key, value)) in self.entries.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{key:?}: {value:?}")?;
        }
        f.write_str("}")
    }
}
