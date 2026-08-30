# CBOR Next

Library to encode and decode a Concise Binary Object Representation (CBOR)

|                    License                     |              Crates Version               |                 Docs                 |
| :--------------------------------------------: | :---------------------------------------: | :----------------------------------: |
| [![License: MIT][license_badge]][license_link] | [![Crate][cratesio_badge]][cratesio_link] | [![Docs][docsrs_badge]][docsrs_link] |

## Features

- Complete support for all major CBOR types (unsigned, signed, floating integers, bytes, text, array, map, tag, boolean, null, undefined and other simple value)
- RFC 8949 compliant, keeping the exact length form and chunking of a decoded item so it re-encodes byte for byte
- Configurable `Encoder` and `Decoder`, with deterministic encoding, length and float normalization, nesting limits and duplicate key handling
- CBOR sequences (RFC 8742) through the encoder and decoder
- Diagnostic notation through `Debug` and `Display`

## Installation

Add `cbor_next` to your `Cargo.toml`

```toml
cbor_next = "0.4.0"
```

## Usage

The main building block of the library is the `DataItem` enum, which represents
a CBOR data item of any major type.

### Encoding and decoding

```rust
use cbor_next::DataItem;

let value = DataItem::Unsigned(10_000_000);
let bytes = vec![0x1a, 0x00, 0x98, 0x96, 0x80];

assert_eq!(value.encode(), bytes);
assert_eq!(DataItem::decode(&bytes).unwrap(), value);
```

### Building and reading values

Each variant has exactly one structural accessor, which hands back the content
type holding the data.

```rust
use cbor_next::{DataItem, Get as _};

let item = DataItem::map([
    ("name", DataItem::from("cbor")),
    ("tags", DataItem::array(["fast", "small"])),
]);

assert_eq!(item["name"], DataItem::from("cbor"));
assert_eq!(item["tags"][0], DataItem::from("fast"));
assert_eq!(item.get("missing"), None);

assert_eq!(item["name"].as_text().unwrap().to_text(), "cbor");
assert_eq!(item["tags"].as_array().unwrap().items().len(), 2);
assert_eq!(item.as_map().unwrap().entries().len(), 2);
```

### Configurable encoder

`DataItem::encode` and `DataItem::decode` are the shortcuts for the default
settings. Everything else — writing into an existing buffer, deterministic
output, sequences, decoding limits — is done through an `Encoder` or a
`Decoder`, which is where the settings live.

```rust
use cbor_next::{DataItem, DeterministicMode, Encoder, FloatMode, LengthMode};

let item = DataItem::map([(2, "b"), (1, "a")]);

// sort map keys and force definite lengths
let deterministic = Encoder::new().deterministic(DeterministicMode::Core);
assert_eq!(
    deterministic.encode(&item),
    DataItem::map([(1, "a"), (2, "b")]).encode()
);

// or normalize just one aspect of the output
let definite = Encoder::new().length(LengthMode::Definite);
let wide = Encoder::new().float(FloatMode::Double);
```

### Configurable decoder

`Decoder` holds untrusted input to the profile your application expects.

```rust
use cbor_next::{DataItem, Decoder, DeterministicMode, Error};

let decoder = Decoder::new()
    .max_nesting_depth(16)
    .allow_indefinite(false)
    .require_deterministic(DeterministicMode::Core);

assert!(decoder.decode(&[0xa2, 0x01, 0x02, 0x03, 0x04]).is_ok());
assert_eq!(
    decoder.decode(&[0xa2, 0x03, 0x04, 0x01, 0x02]),
    Err(Error::NotDeterministic)
);
```

### CBOR sequences

```rust
use cbor_next::{DataItem, Decoder, Encoder};

let bytes = Encoder::new().encode_sequence(&[DataItem::from(1), DataItem::from(2)]);
assert_eq!(bytes, [0x01, 0x02]);

let items = Decoder::new().decode_sequence(&bytes).unwrap();
assert_eq!(items, vec![DataItem::from(1), DataItem::from(2)]);

let (first, rest) = Decoder::new().decode_partial(&bytes).unwrap();
assert_eq!(first, DataItem::from(1));
assert_eq!(rest, [0x02]);
```

For other usage check out the docs of the `DataItem` enum and its methods.

[license_badge]: https://img.shields.io/github/license/iamsauravsharma/cbor_next.svg?style=for-the-badge
[license_link]: LICENSE
[cratesio_badge]: https://img.shields.io/crates/v/cbor_next.svg?style=for-the-badge
[cratesio_link]: https://crates.io/crates/cbor_next
[docsrs_badge]: https://img.shields.io/docsrs/cbor_next/latest?style=for-the-badge
[docsrs_link]: https://docs.rs/cbor_next
