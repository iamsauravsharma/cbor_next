#![feature(f16)]
//! Library to handle a Concise Binary Object Representation (CBOR)

/// Module containing different deterministic mode
pub mod deterministic;

/// Module containing different type of error
pub mod error;

/// Module for index
pub mod index;

/// Module for simple number
pub mod simple_number;

/// Module containing a value
pub mod value;

pub use deterministic::DeterministicMode;
pub use index::Get;
pub use simple_number::SimpleNumber;
pub use value::Value;

#[cfg(test)]
mod tests;
