//! Serde support.
//!
//! [`to_data_item`] and [`to_vec`] serialize, [`from_data_item`] and
//! [`from_slice`] deserialize. [`DataItem`](crate::DataItem) itself works with
//! any serde format.
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
//! # Beyond the serde data model
//! - Tags: [`Tagged`], or [`TaggedWith`] for a fixed number;
//!   [`Tag`](crate::Tag) works too.
//! - `undefined`: [`Undefined`]. Other simple values:
//!   [`Simple`](crate::Simple).
//! - Integers outside -2^64 to 2^64 - 1 become bignums; bignums read as `i128`
//!   or `u128`.
//!
//! These fall back to plain structures in other formats. Anything else serde
//! cannot hold fails with [`Error::Unsupported`] rather than being altered, as
//! does a repeated map key.
//!
//! Indefinite lengths are read; output always uses definite lengths. Enums use
//! serde's default (externally tagged) form.

mod de;
mod error;
mod item;
mod ser;
/// Serde types for registered tags
pub mod tags;
mod wrapper;

pub use de::{from_data_item, from_slice};
pub use error::Error;
pub use ser::{to_data_item, to_vec};
pub use wrapper::{Tagged, TaggedWith, Undefined};
