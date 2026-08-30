//! Contents carried by the different [`DataItem`] variants.
//!
//! [`DataItem`]: crate::DataItem
//!
//! Every content type keeps the information required to re-encode a data item
//! byte for byte, which includes whether the original item used a definite or
//! an indefinite length and, for byte and text strings, how the payload was
//! split into chunks.

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
