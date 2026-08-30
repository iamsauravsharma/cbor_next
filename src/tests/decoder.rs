use crate::data_item::DataItem;
use crate::decode::Decoder;
use crate::deterministic::DeterministicMode;
use crate::error::Error;
use crate::tests::hex_bytes;

#[test]
fn default_decoder_matches_decode_method() {
    let bytes = hex_bytes("a26161016162820203");
    assert_eq!(
        Decoder::new().decode(&bytes).unwrap(),
        DataItem::decode(&bytes).unwrap()
    );
}

#[test]
fn nesting_depth_is_configurable() {
    let decoder = Decoder::new().max_nesting_depth(2);
    assert_eq!(
        decoder.decode(&hex_bytes("8101")).unwrap(),
        DataItem::from(vec![1])
    );
    assert_eq!(
        decoder.decode(&hex_bytes("818101")),
        Err(Error::RecursionLimit { limit: 2 })
    );
    // the default limit accepts what the shallow one rejects
    assert!(DataItem::decode(&hex_bytes("818101")).is_ok());
}

#[test]
fn duplicate_keys_can_be_allowed() {
    let bytes = hex_bytes("a201020103");
    assert_eq!(
        DataItem::decode(&bytes),
        Err(Error::DuplicateMapKey(Box::new(DataItem::from(1))))
    );
    let decoder = Decoder::new().allow_duplicate_keys(true);
    // the last value of a repeated key is the one which is kept
    assert_eq!(decoder.decode(&bytes).unwrap(), DataItem::map([(1, 3)]));
}

#[test]
fn indefinite_lengths_can_be_rejected() {
    let decoder = Decoder::new().allow_indefinite(false);
    for hex_cbor in ["9f0102ff", "bf0102ff", "5f4101ff", "7f6161ff"] {
        assert_eq!(
            decoder.decode(&hex_bytes(hex_cbor)),
            Err(Error::IndefiniteNotAllowed),
            "{hex_cbor}"
        );
    }
    assert!(decoder.decode(&hex_bytes("820102")).is_ok());
}

#[test]
fn deterministic_form_can_be_required() {
    let decoder = Decoder::new().require_deterministic(DeterministicMode::Core);
    assert!(decoder.decode(&hex_bytes("a201020304")).is_ok());
    assert_eq!(
        decoder.decode(&hex_bytes("a203040102")),
        Err(Error::NotDeterministic)
    );
    assert_eq!(
        decoder.decode(&hex_bytes("9f0102ff")),
        Err(Error::NotDeterministic)
    );
    // the length first ordering is a different requirement
    let length_first = Decoder::new().require_deterministic(DeterministicMode::LengthFirst);
    let core_only =
        DataItem::map([(DataItem::from(100), "a"), (DataItem::from(false), "b")]).encode();
    assert!(decoder.decode(&core_only).is_ok());
    assert_eq!(
        length_first.decode(&core_only),
        Err(Error::NotDeterministic)
    );
}

#[test]
fn partial_and_sequence_decoding() {
    let bytes = hex_bytes("0102820304");
    let (first, rest) = Decoder::new().decode_partial(&bytes).unwrap();
    assert_eq!(first, DataItem::from(1));
    assert_eq!(rest, hex_bytes("02820304"));

    assert_eq!(
        Decoder::new().decode_sequence(&bytes).unwrap(),
        vec![
            DataItem::from(1),
            DataItem::from(2),
            DataItem::from(vec![3, 4])
        ]
    );
    // a whole item is still required by decode itself
    assert_eq!(
        Decoder::new().decode(&bytes),
        Err(Error::TrailingBytes { count: 4 })
    );
    // and a truncated tail is reported by the sequence decoder
    assert_eq!(
        Decoder::new().decode_sequence(&hex_bytes("0182")),
        Err(Error::UnexpectedEnd { missing: 1 })
    );
}

#[test]
fn huge_declared_length_does_not_allocate() {
    // a head which claims a collection far larger than the input must fail
    // instead of trying to reserve room for it
    assert_eq!(
        DataItem::decode(&hex_bytes("9bffffffffffffffff00")),
        Err(Error::UnexpectedEnd { missing: 1 })
    );
    assert_eq!(
        DataItem::decode(&hex_bytes("bbffffffffffffffff00")),
        Err(Error::UnexpectedEnd { missing: 1 })
    );
    assert_eq!(
        DataItem::decode(&hex_bytes("5bffffffffffffffff00")),
        Err(Error::UnexpectedEnd {
            missing: 18_446_744_073_709_551_614
        })
    );
}
