#![cfg_attr(docsrs, feature(doc_auto_cfg))]
//! Library to handle a Concise Binary Object Representation (CBOR)

/// Module for different type of content
pub mod content;

/// Module containing a data item
pub mod data_item;

/// Module containing deserializer
#[cfg(feature = "serde")]
pub mod de;

/// Module containing different deterministic mode
pub mod deterministic;

/// Module containing different type of error
pub mod error;

/// Module for index
pub mod index;

/// Module containing serializer
#[cfg(feature = "serde")]
pub mod ser;

pub use content::{ArrayContent, ByteContent, MapContent, SimpleValue, TagContent, TextContent};
pub use data_item::DataItem;
#[cfg(feature = "serde")]
pub use de::from_bytes;
pub use deterministic::DeterministicMode;
pub use index::Get;
#[cfg(feature = "serde")]
pub use ser::to_bytes;

#[cfg(test)]
mod tests;
