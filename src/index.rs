use crate::data_item::DataItem;
use crate::index::private::Sealed;

mod private {
    use crate::data_item::DataItem;

    pub trait Sealed {}
    impl Sealed for usize {}
    impl<T> Sealed for T where T: Into<DataItem> {}
}

/// Trait which is used to get a data item from data item
pub trait Get<Idx>
where
    Idx: Sealed,
{
    /// Get a index value
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::{DataItem, Get};
    /// use indexmap::IndexMap;
    ///
    /// let array_value = DataItem::Array(vec![DataItem::Unsigned(10)].into());
    /// let map_val = vec![(DataItem::Unsigned(10), DataItem::Text("abc".into()))];
    /// let map_value = DataItem::from(map_val);
    /// assert_eq!(array_value.get(0), Some(&DataItem::Unsigned(10)));
    /// assert_eq!(array_value.get(2), None);
    /// assert_eq!(
    ///     map_value.get(DataItem::Unsigned(10)),
    ///     Some(&DataItem::Text("abc".into()))
    /// );
    /// assert_eq!(map_value.get(DataItem::Unsigned(11)), None);
    /// ```
    fn get(&self, idx: Idx) -> Option<&Self>;

    /// Get a mutable index value
    /// # Example
    /// ```rust
    /// use cbor_next::{DataItem, Get};
    /// use indexmap::IndexMap;
    ///
    /// let mut array_value = DataItem::Array(vec![DataItem::Unsigned(10)].into());
    /// assert_eq!(array_value.get(0), Some(&DataItem::Unsigned(10)));
    /// *array_value.get_mut(0).unwrap() = DataItem::Unsigned(20);
    /// assert_eq!(array_value.get(0), Some(&DataItem::Unsigned(20)));
    /// ```
    fn get_mut(&mut self, idx: Idx) -> Option<&mut Self>;
}

impl Get<usize> for DataItem {
    fn get(&self, idx: usize) -> Option<&Self> {
        match self {
            Self::Array(a) => a.array().get(idx),
            _ => None,
        }
    }

    fn get_mut(&mut self, idx: usize) -> Option<&mut Self> {
        match self {
            Self::Array(a) => a.array_mut().get_mut(idx),
            _ => None,
        }
    }
}

impl Get<DataItem> for DataItem {
    fn get(&self, idx: DataItem) -> Option<&Self> {
        match self {
            Self::Map(m) => m.map().get(&idx),
            _ => None,
        }
    }

    fn get_mut(&mut self, idx: DataItem) -> Option<&mut Self> {
        match self {
            Self::Map(m) => m.map_mut().get_mut(&idx),
            _ => None,
        }
    }
}

impl<Idx> std::ops::Index<Idx> for DataItem
where
    DataItem: Get<Idx>,
    Idx: Sealed,
{
    type Output = DataItem;

    fn index(&self, index: Idx) -> &Self::Output {
        self.get(index)
            .expect("failed to get value with provided index")
    }
}

impl<Idx> std::ops::IndexMut<Idx> for DataItem
where
    DataItem: Get<Idx>,
    Idx: Sealed,
{
    fn index_mut(&mut self, index: Idx) -> &mut Self::Output {
        self.get_mut(index)
            .expect("failed to get value with provided index")
    }
}
