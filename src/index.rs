//! Looking items up inside arrays and maps.

use crate::data_item::DataItem;
use crate::index::private::Sealed;

mod private {
    use crate::data_item::DataItem;

    pub trait Sealed {}
    impl Sealed for usize {}
    impl Sealed for &'_ DataItem {}
    impl Sealed for &'_ str {}
}

/// Trait which is used to get a data item out of a data item.
///
/// An array is indexed by position, a map by key. A key is given either as a
/// [`DataItem`] or, for the common case of a text key, as a string slice.
pub trait Get<Idx>
where
    Idx: Sealed,
{
    /// Get the item stored at the index, or `None` when the index is absent or
    /// the item is neither an array nor a map.
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::{DataItem, Get};
    ///
    /// let array = DataItem::array([10]);
    /// assert_eq!(array.get(0), Some(&DataItem::from(10)));
    /// assert_eq!(array.get(2), None);
    ///
    /// let map = DataItem::map([("abc", 10)]);
    /// assert_eq!(map.get("abc"), Some(&DataItem::from(10)));
    /// assert_eq!(map.get(&DataItem::from("abc")), Some(&DataItem::from(10)));
    /// assert_eq!(map.get(&DataItem::Unsigned(11)), None);
    /// ```
    fn get(&self, idx: Idx) -> Option<&Self>;

    /// Get the item stored at the index mutably.
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::{DataItem, Get};
    ///
    /// let mut array = DataItem::array([10]);
    /// *array.get_mut(0).unwrap() = DataItem::from(20);
    /// assert_eq!(array.get(0), Some(&DataItem::from(20)));
    /// ```
    fn get_mut(&mut self, idx: Idx) -> Option<&mut Self>;
}

impl Get<usize> for DataItem {
    fn get(&self, idx: usize) -> Option<&Self> {
        self.as_array()?.items().get(idx)
    }

    fn get_mut(&mut self, idx: usize) -> Option<&mut Self> {
        self.as_array_mut()?.items_mut().get_mut(idx)
    }
}

impl Get<&DataItem> for DataItem {
    fn get(&self, idx: &DataItem) -> Option<&Self> {
        self.as_map()?.entries().get(idx)
    }

    fn get_mut(&mut self, idx: &DataItem) -> Option<&mut Self> {
        self.as_map_mut()?.entries_mut().get_mut(idx)
    }
}

impl Get<&str> for DataItem {
    fn get(&self, idx: &str) -> Option<&Self> {
        self.get(&DataItem::from(idx))
    }

    fn get_mut(&mut self, idx: &str) -> Option<&mut Self> {
        self.get_mut(&DataItem::from(idx))
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
