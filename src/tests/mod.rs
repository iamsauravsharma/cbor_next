#![expect(clippy::panic, reason = "allow panic in tests")]

mod decoder;
mod deterministic;
mod diagnostic;
mod encoder;
mod failure;
mod index;
mod roundtrip;

use crate::data_item::DataItem;
use crate::error::Error;

/// Decode a hex string of a test vector into the bytes it stands for
fn hex_bytes(hex_cbor: &str) -> Vec<u8> {
    hex::decode(hex_cbor).unwrap_or_else(|err| panic!("{err} failed to decode hex {hex_cbor}"))
}

/// Check that a value encodes to the provided `CBOR` bytes
fn encode_compare<I>(hex_cbor: &str, value_into: I)
where
    I: Into<DataItem>,
{
    let value = value_into.into();
    assert_eq!(value.encode(), hex_bytes(hex_cbor), "{hex_cbor}");
}

/// Check that the provided `CBOR` bytes decode to a value
fn decode_compare<I>(hex_cbor: &str, value_into: I)
where
    I: Into<DataItem>,
{
    let value = value_into.into();
    let decoded = DataItem::decode(&hex_bytes(hex_cbor))
        .unwrap_or_else(|err: Error| panic!("{err} failed to decode value {hex_cbor}"));
    assert_eq!(decoded, value, "{hex_cbor}");
}

/// Check that a value encodes to the provided `CBOR` bytes and that those bytes
/// decode back to a value which encodes the same way
fn compare_cbor_value<I>(hex_cbor: &str, value_into: I)
where
    I: Into<DataItem>,
{
    let value = value_into.into();
    let bytes = hex_bytes(hex_cbor);
    assert_eq!(value.encode(), bytes, "{hex_cbor}");
    let decoded = DataItem::decode(&bytes)
        .unwrap_or_else(|err| panic!("{err} failed to decode value {hex_cbor}"));
    assert_eq!(decoded, value, "{hex_cbor}");
    assert_eq!(decoded.encode(), bytes, "{hex_cbor}");
}
