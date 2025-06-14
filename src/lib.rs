//! Library to handle a Concise Binary Object Representation (CBOR)

/// Module containing a data item
pub mod data_item;

/// Module containing different deterministic mode
pub mod deterministic;

/// Module containing different type of error
pub mod error;

/// Module for index
pub mod index;

/// Module for simple number
pub mod simple_number;

pub use data_item::DataItem;
pub use deterministic::DeterministicMode;
pub use index::Get;
pub use simple_number::SimpleNumber;

#[cfg(test)]
mod tests;
