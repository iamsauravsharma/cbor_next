use crate::decode::Decoder;
use crate::error::Error;
use crate::stream::{StreamDecoder, Token};
use crate::tests::hex_bytes;

/// Feed the bytes one at a time and gather every token.
fn tokens_one_byte_at_a_time(hex_cbor: &str) -> Result<Vec<Token>, Error> {
    let mut stream = Decoder::new().stream();
    let mut tokens = vec![];
    for byte in hex_bytes(&hex_cbor.replace(' ', "")) {
        stream.feed(&[byte]);
        while let Some(token) = stream.next_token()? {
            tokens.push(token);
        }
    }
    assert!(
        stream.is_at_boundary(),
        "{hex_cbor} did not end at a boundary"
    );
    Ok(tokens)
}

fn tokens_at_once(hex_cbor: &str) -> Result<Vec<Token>, Error> {
    let mut stream = Decoder::new().stream();
    stream.feed(&hex_bytes(&hex_cbor.replace(' ', "")));
    let mut tokens = vec![];
    while let Some(token) = stream.next_token()? {
        tokens.push(token);
    }
    Ok(tokens)
}

#[test]
fn every_head_is_a_token() {
    // {1: [2, 3], "a": (_ h'01', h'02'), 2: 1(-1), 3: [_ 1.5, null, true,
    // undefined, simple(16), -Infinity]}
    let hex_cbor =
        "a4 01 82 02 03 6161 5f 4101 4102 ff 02 c1 20 03 9f f93e00 f6 f5 f7 f0 f9fc00 ff";
    let expected = vec![
        Token::MapStart(Some(4)),
        Token::Unsigned(1),
        Token::ArrayStart(Some(2)),
        Token::Unsigned(2),
        Token::Unsigned(3),
        Token::Text("a".into()),
        Token::BytesStart,
        Token::Bytes(vec![1]),
        Token::Bytes(vec![2]),
        Token::Break,
        Token::Unsigned(2),
        Token::Tag(1),
        Token::Signed(0),
        Token::Unsigned(3),
        Token::ArrayStart(None),
        Token::Floating(1.5),
        Token::Null,
        Token::Boolean(true),
        Token::Undefined,
        Token::Simple(crate::content::Simple::try_from(16).unwrap()),
        Token::Floating(f64::NEG_INFINITY),
        Token::Break,
    ];
    assert_eq!(tokens_one_byte_at_a_time(hex_cbor).unwrap(), expected);
    assert_eq!(tokens_at_once(hex_cbor).unwrap(), expected);
}

#[test]
fn a_sequence_reaches_a_boundary_after_each_item() {
    let mut stream = Decoder::new().stream();
    stream.feed(&hex_bytes("01a1008001"));
    assert_eq!(stream.next_token().unwrap(), Some(Token::Unsigned(1)));
    assert!(!stream.is_at_boundary());
    assert_eq!(stream.next_token().unwrap(), Some(Token::MapStart(Some(1))));
    assert_eq!(stream.depth(), 1);
    assert_eq!(stream.next_token().unwrap(), Some(Token::Unsigned(0)));
    assert_eq!(
        stream.next_token().unwrap(),
        Some(Token::ArrayStart(Some(0)))
    );
    assert_eq!(stream.depth(), 0);
    assert_eq!(stream.pending(), 1);
    assert_eq!(stream.next_token().unwrap(), Some(Token::Unsigned(1)));
    assert!(stream.is_at_boundary());
    assert_eq!(stream.next_token().unwrap(), None);
    // an empty stream is at a boundary as well
    assert!(Decoder::new().stream().is_at_boundary());
    // a tag completes with its content, however deep
    let mut stream = Decoder::new().stream();
    stream.feed(&hex_bytes("81c1c101"));
    for _ in 0..4 {
        stream.next_token().unwrap().unwrap();
    }
    assert!(stream.is_at_boundary());
}

#[test]
fn a_truncated_item_waits_for_more_bytes() {
    let mut stream = Decoder::new().stream();
    for (bytes, token) in [
        (vec![0x19, 0x01], None),
        (vec![0x00], Some(Token::Unsigned(256))),
        (vec![0x62, 0x61], None),
        (vec![0x62], Some(Token::Text("ab".into()))),
        (vec![0xfb, 0, 0, 0, 0, 0, 0, 0], None),
        (vec![0], Some(Token::Floating(0.0))),
        (vec![0xf8], None),
        (
            vec![0xff],
            Some(Token::Simple(
                crate::content::Simple::try_from(255).unwrap(),
            )),
        ),
    ] {
        stream.feed(&bytes);
        assert_eq!(stream.next_token().unwrap(), token);
    }
    assert!(stream.is_at_boundary());
    // an open container is not a boundary either
    let mut stream = Decoder::new().stream();
    stream.feed(&[0x9f]);
    stream.next_token().unwrap();
    assert!(!stream.is_at_boundary());
    assert_eq!(stream.next_token().unwrap(), None);
}

#[test]
fn not_well_formed_input_is_rejected() {
    for (hex_cbor, check) in [
        ("ff", "break"),
        ("81ff", "break"),
        ("a100ff", "break"),
        ("bf00ff", "break"),
        ("bf000000ff", "break"),
        ("c0ff", "break"),
        ("5f00ff", "chunk"),
        ("7f4100ff", "chunk"),
        ("5f5fffff", "chunk"),
        ("1c", "additional"),
        ("1f", "additional"),
        ("3f", "additional"),
        ("df", "additional"),
        ("fc", "additional"),
        ("f800", "simple"),
        ("f81f", "simple"),
        ("7f61c361a9ff", "utf8"),
        ("62c328", "utf8"),
    ] {
        let error = tokens_at_once(hex_cbor).expect_err(hex_cbor);
        let right_kind = match check {
            "break" => matches!(error, Error::UnexpectedBreak),
            "chunk" => {
                matches!(
                    error,
                    Error::InvalidChunkMajorType { .. } | Error::InvalidAdditionalInfo { .. }
                )
            }
            "additional" => matches!(error, Error::InvalidAdditionalInfo { .. }),
            "simple" => matches!(error, Error::InvalidSimpleValue(_)),
            _ => matches!(error, Error::InvalidUtf8(_)),
        };
        assert!(right_kind, "{hex_cbor}: {error}");
    }
    // nothing is decoded after an error
    let mut stream = Decoder::new().stream();
    stream.feed(&hex_bytes("ff01"));
    assert!(stream.next_token().is_err());
    assert_eq!(stream.next_token().unwrap(), None);
    assert!(!stream.is_at_boundary());
}

#[test]
fn limits_of_the_decoder_apply() {
    let mut stream = Decoder::new().max_nesting_depth(2).stream();
    stream.feed(&hex_bytes("818101"));
    assert_eq!(
        stream.next_token().unwrap(),
        Some(Token::ArrayStart(Some(1)))
    );
    assert_eq!(stream.next_token(), Err(Error::RecursionLimit { limit: 2 }));
    let mut stream = Decoder::new().max_items(2).stream();
    stream.feed(&hex_bytes("010203"));
    stream.next_token().unwrap();
    stream.next_token().unwrap();
    assert_eq!(stream.next_token(), Err(Error::ItemLimit { limit: 2 }));
    // a break is not an item, as in the decoder
    let decoder = Decoder::new().max_items(2);
    let bytes = hex_bytes("9f01ff");
    assert!(decoder.decode(&bytes).is_ok());
    let mut stream = decoder.stream();
    stream.feed(&bytes);
    while stream.next_token().unwrap().is_some() {}
    assert!(stream.is_at_boundary());
    let mut stream = Decoder::new().allow_indefinite(false).stream();
    stream.feed(&hex_bytes("9f"));
    assert_eq!(stream.next_token(), Err(Error::IndefiniteNotAllowed));
}

#[test]
fn handed_out_bytes_are_let_go_of() {
    let mut stream: StreamDecoder = Decoder::new().stream();
    let mut count = 0;
    for _ in 0..10_000 {
        stream.feed(&hex_bytes("1864"));
        while let Some(token) = stream.next_token().unwrap() {
            assert_eq!(token, Token::Unsigned(100));
            count += 1;
        }
    }
    assert_eq!(count, 10_000);
    assert!(stream.is_at_boundary());
    assert_eq!(stream.pending(), 0);
}

#[test]
fn float_payloads_survive() {
    let tokens = tokens_at_once("f97c01 fa7f800001").unwrap();
    let [Token::Floating(half), Token::Floating(single)] = tokens.as_slice() else {
        panic!("expected two floats");
    };
    assert_eq!(half.to_bits(), 0x7ff0_0400_0000_0000);
    assert_eq!(single.to_bits(), 0x7ff0_0000_2000_0000);
}

#[test]
fn tokens_are_written_back_to_the_same_bytes() {
    use crate::stream::StreamEncoder;

    let hex_cbor =
        "a4 01 82 02 03 6161 5f 4101 4102 ff 02 c1 20 03 9f f93e00 f6 f5 f7 f0 f9fc00 ff";
    let tokens = tokens_at_once(hex_cbor).unwrap();
    let mut encoder = StreamEncoder::new();
    for token in &tokens {
        encoder.push(token).unwrap();
    }
    assert!(encoder.is_at_boundary());
    assert_eq!(encoder.bytes(), hex_bytes(&hex_cbor.replace(' ', "")));
    // taking the bytes leaves the encoder ready for more
    assert_eq!(encoder.take(), hex_bytes(&hex_cbor.replace(' ', "")));
    encoder.push(&Token::Unsigned(300)).unwrap();
    assert_eq!(encoder.take(), hex_bytes("19012c"));
    assert_eq!(encoder.bytes(), [0_u8; 0]);
}

#[test]
fn an_encoder_refuses_what_would_not_be_well_formed() {
    use crate::stream::StreamEncoder;

    let mut encoder = StreamEncoder::new();
    assert_eq!(encoder.push(&Token::Break), Err(Error::UnexpectedBreak));
    encoder.push(&Token::MapStart(None)).unwrap();
    encoder.push(&Token::Unsigned(1)).unwrap();
    // a break in a value position, then the value, then a proper break
    assert_eq!(encoder.push(&Token::Break), Err(Error::UnexpectedBreak));
    encoder.push(&Token::Unsigned(2)).unwrap();
    encoder.push(&Token::Break).unwrap();
    assert!(encoder.is_at_boundary());
    // only chunks of the right type inside an indefinite length string
    encoder.push(&Token::BytesStart).unwrap();
    assert_eq!(
        encoder.push(&Token::Text("a".into())),
        Err(Error::InvalidChunkMajorType {
            expected: 2,
            found: 3
        })
    );
    assert_eq!(
        encoder.push(&Token::Unsigned(1)),
        Err(Error::InvalidChunkMajorType {
            expected: 2,
            found: 0
        })
    );
    encoder.push(&Token::Bytes(vec![1])).unwrap();
    encoder.push(&Token::Break).unwrap();
    // a definite count closes on its own, and a tag with its content
    encoder.push(&Token::Tag(1)).unwrap();
    encoder.push(&Token::ArrayStart(Some(1))).unwrap();
    assert_eq!(encoder.depth(), 2);
    encoder.push(&Token::Unsigned(0)).unwrap();
    assert!(encoder.is_at_boundary());
    assert_eq!(encoder.take(), hex_bytes("bf0102ff5f4101ffc18100"));
    // the nesting limit
    let mut shallow = StreamEncoder::new().max_nesting_depth(2);
    shallow.push(&Token::ArrayStart(Some(1))).unwrap();
    assert_eq!(
        shallow.push(&Token::ArrayStart(Some(1))),
        Err(Error::RecursionLimit { limit: 2 })
    );
    // nothing was written for a refused token
    assert_eq!(shallow.bytes(), [0x81]);
}
