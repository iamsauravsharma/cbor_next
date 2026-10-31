//! Contents of the [`DataItem`](crate::DataItem) variants.
//!
//! Each keeps its length form and chunks, so a decoded item re-encodes byte
//! for byte.

mod array;
mod byte;
mod map;
mod simple;
mod tag;
mod text;

pub use array::Array;
pub use byte::ByteString;
pub use map::Map;
pub use simple::Simple;
pub use tag::Tag;
pub use text::TextString;
