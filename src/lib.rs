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

pub use content::SimpleValue;
pub use data_item::DataItem;
pub use deterministic::DeterministicMode;
pub use index::Get;

#[cfg(test)]
mod tests;
