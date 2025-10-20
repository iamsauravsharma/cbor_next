#![cfg_attr(docsrs, feature(doc_cfg))]
//! Library to handle a Concise Binary Object Representation (CBOR)

/// Module for different type of content
pub mod content;

/// Module containing a data item
pub mod data_item;

/// Module containing different deterministic mode
pub mod deterministic;

/// Module containing different type of error
pub mod error;

/// Module for index
pub mod index;

#[doc(inline)]
pub use content::{ArrayContent, ByteContent, MapContent, SimpleValue, TagContent, TextContent};
#[doc(inline)]
pub use data_item::DataItem;
#[doc(inline)]
pub use deterministic::DeterministicMode;
#[doc(inline)]
pub use index::Get;

#[cfg(test)]
mod tests;
