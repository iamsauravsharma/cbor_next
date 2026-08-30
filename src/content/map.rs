use std::fmt::{Debug, Formatter, Result as FmtResult};

use indexmap::IndexMap;

use crate::data_item::DataItem;

/// Content of a map, `CBOR` major type 5.
///
/// Entries are kept in their original order by an [`IndexMap`], so a decoded
/// map re-encodes to the same bytes, while comparison and hashing stay
/// independent of that order. A map also remembers whether it was encoded with
/// a definite length or with the indefinite length form terminated by a break.
/// The entries themselves are reached through [`entries`](Map::entries) and its
/// mutable and owning counterparts.
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
#[derive(Default, PartialEq, Eq, Clone)]
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

    /// Create a definite length map from a sequence of key value pairs.
    ///
    /// A key which is present more than once keeps the position of its first
    /// occurrence and the value of its last one.
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

    /// Return the same content with the definite or indefinite encoding
    /// selected.
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

    /// Select the definite or indefinite encoding in place.
    pub fn set_indefinite(&mut self, indefinite: bool) {
        self.is_indefinite = indefinite;
    }

    /// Whether the map uses the indefinite length encoding.
    #[must_use]
    pub fn is_indefinite(&self) -> bool {
        self.is_indefinite
    }

    /// The entries of the map in encoding order.
    #[must_use]
    pub fn entries(&self) -> &IndexMap<DataItem, DataItem> {
        &self.entries
    }

    /// The entries of the map, mutably.
    #[must_use]
    pub fn entries_mut(&mut self) -> &mut IndexMap<DataItem, DataItem> {
        &mut self.entries
    }

    /// Take ownership of the entries of the map.
    #[must_use]
    pub fn into_entries(self) -> IndexMap<DataItem, DataItem> {
        self.entries
    }
}

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
