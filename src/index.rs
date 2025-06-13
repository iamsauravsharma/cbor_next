use crate::index::private::Sealed;
use crate::value::Value;

mod private {
    use crate::value::Value;

    pub trait Sealed {}
    impl Sealed for usize {}
    impl<T> Sealed for T where T: Into<Value> {}
}

/// Trait which is used to get a value
pub trait Get<Idx>
where
    Idx: Sealed,
{
    /// Get a index value
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::{Get, Value};
    /// use indexmap::IndexMap;
    ///
    /// let array_value = Value::Array(vec![Value::Unsigned(10)]);
    /// let map_value = Value::Map(IndexMap::from_iter(vec![(
    ///     Value::Unsigned(10),
    ///     Value::Text("abc".to_string()),
    /// )]));
    /// assert_eq!(array_value.get(0), Some(&Value::Unsigned(10)));
    /// assert_eq!(array_value.get(2), None);
    /// assert_eq!(
    ///     map_value.get(Value::Unsigned(10)),
    ///     Some(&Value::Text("abc".to_string()))
    /// );
    /// assert_eq!(map_value.get(Value::Unsigned(11)), None);
    /// ```
    fn get(&self, idx: Idx) -> Option<&Self>;

    /// Get a mutable index value
    /// # Example
    /// ```rust
    /// use cbor_next::{Get, Value};
    /// use indexmap::IndexMap;
    ///
    /// let mut array_value = Value::Array(vec![Value::Unsigned(10)]);
    /// assert_eq!(array_value.get(0), Some(&Value::Unsigned(10)));
    /// *array_value.get_mut(0).unwrap() = Value::Unsigned(20);
    /// assert_eq!(array_value.get(0), Some(&Value::Unsigned(20)));
    /// ```
    fn get_mut(&mut self, idx: Idx) -> Option<&mut Self>;
}

impl Get<usize> for Value {
    fn get(&self, idx: usize) -> Option<&Self> {
        match self {
            Value::Array(a) => a.get(idx),
            _ => None,
        }
    }

    fn get_mut(&mut self, idx: usize) -> Option<&mut Self> {
        match self {
            Value::Array(a) => a.get_mut(idx),
            _ => None,
        }
    }
}

impl Get<Value> for Value {
    fn get(&self, idx: Value) -> Option<&Self> {
        match self {
            Value::Map(a) => a.get(&idx),
            _ => None,
        }
    }

    fn get_mut(&mut self, idx: Value) -> Option<&mut Self> {
        match self {
            Value::Map(a) => a.get_mut(&idx),
            _ => None,
        }
    }
}

impl<Idx> std::ops::Index<Idx> for Value
where
    Value: Get<Idx>,
    Idx: Sealed,
{
    type Output = Value;

    fn index(&self, index: Idx) -> &Self::Output {
        self.get(index)
            .expect("failed to get value with provided index")
    }
}

impl<Idx> std::ops::IndexMut<Idx> for Value
where
    Value: Get<Idx>,
    Idx: Sealed,
{
    fn index_mut(&mut self, index: Idx) -> &mut Self::Output {
        self.get_mut(index)
            .expect("failed to get value with provided index")
    }
}
