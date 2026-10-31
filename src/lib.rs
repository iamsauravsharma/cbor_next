#![cfg_attr(docsrs, feature(doc_cfg))]
//! Concise Binary Object Representation (`CBOR`, RFC 8949) for Rust.
//!
//! Every value is a [`DataItem`]: [`encode`](DataItem::encode) it and
//! [`decode`](DataItem::decode) it back.
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
//! [`Encoder`] and [`Decoder`] do the same with custom settings, such as
//! deterministic output or limits for untrusted input.
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
//!     .max_items(1_000)
//!     .require_deterministic(DeterministicMode::Core);
//! assert_eq!(
//!     decoder.decode(&encoded).unwrap(),
//!     DataItem::map([(1, "a"), (2, "b")])
//! );
//! ```

/// Contents of the data item variants
pub mod content;

/// The data item type
pub mod data_item;

/// Decoding
pub mod decode;

/// Deterministic encoding modes
pub mod deterministic;

/// Encoding
pub mod encode;

/// Errors
pub mod error;

/// Indexing into arrays and maps
pub mod index;

/// Registered tag numbers
pub mod tags;

/// Serde support
#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
pub mod serde;

#[doc(inline)]
pub use content::{Array, ByteString, Map, Simple, Tag, TextString};
#[doc(inline)]
pub use data_item::DataItem;
#[doc(inline)]
pub use decode::{Decoder, DuplicateKeyMode, Sequence};
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
