# CBOR Next

Library to encode and decode a Concise Binary Object Representation (CBOR)

|                    License                     |              Crates Version               |                 Docs                 |
| :--------------------------------------------: | :---------------------------------------: | :----------------------------------: |
| [![License: MIT][license_badge]][license_link] | [![Crate][cratesio_badge]][cratesio_link] | [![Docs][docsrs_badge]][docsrs_link] |

## Features

- Complete support for all major CBOR types (unsigned, signed, floating integers, bytes, text, array, map, tag, boolean, null, undefined and other simple value)
- RFC 8949 compliant
- Deterministic encoding support via `DeterministicMode`
- Encoding/Decoding to and from CBOR binary format

## Installation

Add `cbor_next` to your `Cargo.toml`

```toml
cbor_next = "0.1.0"
```

## Usage

Main building block of library is `Value` Enum which represent a different type CBOR value

### Encoding
```rust
use cbor_next::Value;
let value = Value::Unsigned(10_000_000);
let vector_data = vec![0x1a, 0x00, 0x98, 0x96, 0x80];
assert_eq!(value.encode(), vector_data);
```

### Decoding
```rust
use cbor_next::Value;
let vector_data = vec![0x1a, 0x00, 0x98, 0x96, 0x80];
let value = Value::Unsigned(10_000_000);
assert_eq!(Value::decode(&vector_data).unwrap(), value);
```

For other usage check out docs `Value` enum and its methods and functions

[license_badge]: https://img.shields.io/github/license/iamsauravsharma/cbor_next.svg?style=for-the-badge
[license_link]: LICENSE
[cratesio_badge]: https://img.shields.io/crates/v/cbor_next.svg?style=for-the-badge
[cratesio_link]: https://crates.io/crates/cbor_next
[docsrs_badge]: https://img.shields.io/docsrs/cbor_next/latest?style=for-the-badge
[docsrs_link]: https://docs.rs/cbor_next
