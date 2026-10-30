//! Converting between Rust values and data items with serde.
//!
//! [`to_data_item`] and [`to_vec`] write any value which implements
//! `Serialize`, and [`from_data_item`] and [`from_slice`] read one which
//! implements `Deserialize`. A [`DataItem`](crate::DataItem) implements both
//! traits itself, so it can also be written to or read from another format.
//!
//! ```rust
//! use cbor_next::serde::{from_slice, to_vec};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize, Debug, PartialEq)]
//! struct Point {
//!     x: i32,
//!     y: i32,
//! }
//!
//! let bytes = to_vec(&Point { x: 1, y: -2 }).unwrap();
//! // a map of two entries: {"x": 1, "y": -2}
//! assert_eq!(bytes, [0xa2, 0x61, 0x78, 0x01, 0x61, 0x79, 0x21]);
//! assert_eq!(from_slice::<Point>(&bytes).unwrap(), Point { x: 1, y: -2 });
//! ```
//!
//! # Data serde has no counterpart for
//! `CBOR` holds a few things the serde data model does not. Each one has a
//! type which carries it through serde unchanged, and which falls back to a
//! plain structure through any other format:
//! - a tag is written and read with [`Tagged`], or with [`TaggedWith`] when the
//!   tag number is fixed by the protocol; a [`Tag`](crate::Tag) implements the
//!   traits the same way
//! - `undefined` is written and read with [`Undefined`]
//! - any other simple value is written and read with [`Simple`](crate::Simple)
//! - an integer outside the range of `CBOR`, -2^64 to 2^64 - 1, is written as a
//!   bignum, tag 2 or 3 of RFC 8949 section 3.4.3, and a bignum is read as an
//!   `i128` or `u128`
//! - a map key may be any value, as in both data models
//!
//! Anything which falls outside of that is rejected with
//! [`Error::Unsupported`], never altered to fit: a tag which nothing asked
//! for, `undefined` or a simple value in place of another type, a bignum
//! wider than 128 bits and a map which uses the same key twice, which RFC
//! 8949 section 5.6 calls invalid.
//!
//! Indefinite length strings, arrays and maps are accepted when reading, their
//! chunks joined, and values are always written with a definite length, the
//! preferred serialization once the length is known. An enum is written the
//! way serde does by default: a unit variant as its name and any other
//! variant as a map of one entry from its name to its content.

mod de;
mod error;
mod item;
mod ser;
mod wrapper;

pub use de::{from_data_item, from_slice};
pub use error::Error;
pub use ser::{to_data_item, to_vec};
pub use wrapper::{Tagged, TaggedWith, Undefined};
