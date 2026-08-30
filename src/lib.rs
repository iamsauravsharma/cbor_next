#![cfg_attr(docsrs, feature(doc_cfg))]
//! Library to handle a Concise Binary Object Representation (`CBOR`)
//!
//! Every `CBOR` value is a [`DataItem`], which is encoded with
//! [`DataItem::encode`] and decoded with [`DataItem::decode`].
//!
//! ```rust
//! use cbor_next::{DataItem, Get as _, TextString};
//!
//! let item = DataItem::map([("name", "cbor"), ("kind", "format")]);
//! let encoded = item.encode();
//!
//! let decoded = DataItem::decode(&encoded).unwrap();
//! assert_eq!(
//!     decoded
//!         .get("name")
//!         .and_then(DataItem::as_text)
//!         .map(TextString::to_text)
//!         .unwrap(),
//!     "cbor"
//! );
//! ```
//!
//! Those two methods use the default settings. [`Encoder`] and [`Decoder`] do
//! the same work under settings of your choosing, which is how deterministic
//! output is produced and how untrusted input is held to a profile.
//!
//! ```rust
//! use cbor_next::{DataItem, Decoder, DeterministicMode, Encoder};
//!
//! let item = DataItem::map([(2, "b"), (1, "a")]);
//! let encoded = Encoder::new()
//!     .deterministic(DeterministicMode::Core)
//!     .encode(&item);
//!
//! let decoder = Decoder::new()
//!     .max_nesting_depth(16)
//!     .require_deterministic(DeterministicMode::Core);
//! assert_eq!(
//!     decoder.decode(&encoded).unwrap(),
//!     DataItem::map([(1, "a"), (2, "b")])
//! );
//! ```

/// Module for the content held by the different data item variants
pub mod content;

/// Module containing a data item
pub mod data_item;

/// Module for decoding `CBOR` bytes
pub mod decode;

/// Module containing different deterministic mode
pub mod deterministic;

/// Module for encoding data items
pub mod encode;

/// Module containing different type of error
pub mod error;

/// Module for index
pub mod index;

#[doc(inline)]
pub use content::{Array, ByteString, Map, Simple, Tag, TextString};
#[doc(inline)]
pub use data_item::DataItem;
#[doc(inline)]
pub use decode::Decoder;
#[doc(inline)]
pub use deterministic::DeterministicMode;
#[doc(inline)]
pub use encode::{Encoder, FloatMode, LengthMode};
#[doc(inline)]
pub use error::Error;
#[doc(inline)]
pub use index::Get;

#[cfg(test)]
mod tests;
