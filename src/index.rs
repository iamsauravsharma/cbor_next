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
    fn get(&self, idx: Idx) -> Option<&Self>;
    /// Get a mutable index value
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

impl<T> Get<T> for Value
where
    T: Into<Value>,
{
    fn get(&self, idx: T) -> Option<&Self> {
        match self {
            Value::Map(a) => a.get(&idx.into()),
            _ => None,
        }
    }

    fn get_mut(&mut self, idx: T) -> Option<&mut Self> {
        match self {
            Value::Map(a) => a.get_mut(&idx.into()),
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
