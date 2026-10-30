use crate::content::{Array, TextString};
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
fn deterministic_form_requires_shortest_encoding() {
    let decoder = Decoder::new().require_deterministic(DeterministicMode::Core);
    // heads written wider than they need to be, for every major type with an
    // argument
    for hex_cbor in [
        "1805",               // unsigned integer 5 as one byte
        "190005",             // 5 as two bytes
        "1a00000005",         // 5 as four bytes
        "1b0000000000000005", // 5 as eight bytes
        "190018",             // 24 as two bytes
        "1900ff",             // 255 as two bytes
        "1a0000ffff",         // 65535 as four bytes
        "1b00000000ffffffff", // 4294967295 as eight bytes
        "3805",               // negative integer
        "5801aa",             // byte string length
        "780161",             // text string length
        "980100",             // array length
        "b8010000",           // map length
        "d8010a",             // tag number
        "8218010a",           // a wide head nested inside an array
    ] {
        assert_eq!(
            decoder.decode(&hex_bytes(hex_cbor)),
            Err(Error::NotDeterministic),
            "{hex_cbor}"
        );
        // the default decoder keeps accepting them, only the requirement
        // rejects them
        assert!(DataItem::decode(&hex_bytes(hex_cbor)).is_ok(), "{hex_cbor}");
    }
    // the shortest form of the same kinds of values is accepted
    for hex_cbor in [
        "05",
        "1818",
        "18ff",
        "190100",
        "19ffff",
        "1a00010000",
        "1affffffff",
        "1b0000000100000000",
        "8101",
    ] {
        assert!(decoder.decode(&hex_bytes(hex_cbor)).is_ok(), "{hex_cbor}");
    }
    // a float which fits a narrower width has to use it
    assert_eq!(
        decoder.decode(&hex_bytes("fa3fc00000")),
        Err(Error::NotDeterministic),
        "1.5 as a single"
    );
    assert_eq!(
        decoder.decode(&hex_bytes("fb3ff8000000000000")),
        Err(Error::NotDeterministic),
        "1.5 as a double"
    );
    assert!(
        decoder.decode(&hex_bytes("f93e00")).is_ok(),
        "1.5 as a half"
    );
    assert!(
        decoder.decode(&hex_bytes("fa47c35000")).is_ok(),
        "100000.0 as a single"
    );
    assert!(
        decoder.decode(&hex_bytes("fb3ff199999999999a")).is_ok(),
        "1.1 as a double"
    );
}

#[test]
fn nan_equals_itself_and_repeated_nan_key_is_a_duplicate() {
    let nan = DataItem::decode(&hex_bytes("f97e00")).unwrap();
    assert_eq!(nan, nan.clone());
    // a different payload is a different value
    assert_ne!(nan, DataItem::decode(&hex_bytes("f97e01")).unwrap());
    // RFC 8949 section 5.6.1: -0.0 is equal to 0.0
    assert_eq!(DataItem::from(0.0), DataItem::from(-0.0));
    // NaNs are equivalent by significand, whatever their sign
    assert_eq!(nan, DataItem::decode(&hex_bytes("f9fe00")).unwrap());
    // a half precision significand widens to the same double significand
    assert_eq!(nan, DataItem::decode(&hex_bytes("fa7fc00000")).unwrap());
    // a map holding a NaN key equals its own copy
    let map = DataItem::decode(&hex_bytes("a1f97e0001")).unwrap();
    assert_eq!(map, map.clone());
    // the same NaN used for two keys is a repeated key
    assert!(matches!(
        DataItem::decode(&hex_bytes("a2f97e0001f97e0002")),
        Err(Error::DuplicateMapKey(_))
    ));
}

#[test]
fn item_count_can_be_limited() {
    let decoder = Decoder::new().max_items(3);
    // the array and its two elements are three items
    assert!(decoder.decode(&hex_bytes("820102")).is_ok());
    assert_eq!(
        decoder.decode(&hex_bytes("83010203")),
        Err(Error::ItemLimit { limit: 3 })
    );
    // a map counts its keys and its values, a tag counts its content
    assert!(decoder.decode(&hex_bytes("a10102")).is_ok());
    assert_eq!(
        decoder.decode(&hex_bytes("a201020304")),
        Err(Error::ItemLimit { limit: 3 })
    );
    assert!(decoder.decode(&hex_bytes("c101")).is_ok());
    assert_eq!(
        decoder.decode(&hex_bytes("81818101")),
        Err(Error::ItemLimit { limit: 3 })
    );
    // indefinite lengths count their items too
    assert_eq!(
        decoder.decode(&hex_bytes("9f01020304ff")),
        Err(Error::ItemLimit { limit: 3 })
    );
    // every chunk of an indefinite length string counts too, so a string made
    // of many chunks cannot get around the limit
    assert!(decoder.decode(&hex_bytes("5f4101ff")).is_ok());
    assert!(decoder.decode(&hex_bytes("5f41014102ff")).is_ok());
    assert_eq!(
        decoder.decode(&hex_bytes("5f410141024103ff")),
        Err(Error::ItemLimit { limit: 3 })
    );
    assert_eq!(
        decoder.decode(&hex_bytes("7f616161626163ff")),
        Err(Error::ItemLimit { limit: 3 })
    );
    // the string itself is one item, however many empty chunks follow
    let mut empty_chunks = vec![0x5f];
    empty_chunks.extend(std::iter::repeat_n(0x40, 1000));
    empty_chunks.push(0xff);
    assert_eq!(
        Decoder::new().max_items(1).decode(&empty_chunks),
        Err(Error::ItemLimit { limit: 1 })
    );
    assert!(
        Decoder::new()
            .max_items(1)
            .decode(&hex_bytes("5fff"))
            .is_ok()
    );
    let mut empty_text_chunks = vec![0x7f];
    empty_text_chunks.extend(std::iter::repeat_n(0x60, 1000));
    empty_text_chunks.push(0xff);
    assert_eq!(
        Decoder::new().max_items(1).decode(&empty_text_chunks),
        Err(Error::ItemLimit { limit: 1 })
    );
    // one limit is shared by a whole sequence
    assert!(decoder.decode_sequence(&hex_bytes("010203")).is_ok());
    assert_eq!(
        decoder.decode_sequence(&hex_bytes("01020304")),
        Err(Error::ItemLimit { limit: 3 })
    );
    // a head which announces far more items than the limit fails once the
    // limit is passed
    let mut many = hex_bytes("9a00ffffff");
    many.extend([0; 20]);
    assert_eq!(
        Decoder::new().max_items(10).decode(&many),
        Err(Error::ItemLimit { limit: 10 })
    );
    // nothing is limited unless a limit is set
    assert!(DataItem::decode(&hex_bytes("850102030405")).is_ok());
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

#[test]
fn duplicate_key_is_found_across_length_forms() {
    // "hello" and the chunks "he" and "llo"
    let bytes = hex_bytes("a26568656c6c6f017f626865636c6c6fff02");
    assert!(matches!(
        DataItem::decode(&bytes),
        Err(Error::DuplicateMapKey(_))
    ));
}

#[test]
fn zero_and_negative_zero_are_duplicate_keys() {
    assert!(matches!(
        DataItem::decode(&hex_bytes("a2f9000001f9800002")),
        Err(Error::DuplicateMapKey(_))
    ));
    // an integer, a float and a bignum of one number are distinct
    let distinct = DataItem::decode(&hex_bytes(
        "a300 01f90000 02c24100 03".replace(' ', "").as_str(),
    ));
    assert_eq!(distinct.unwrap().as_map().unwrap().entries().len(), 3);
}

#[test]
fn length_form_does_not_change_equality() {
    let definite = DataItem::from(vec![1, 2]);
    let indefinite = DataItem::Array(Array::from_items([1, 2]).with_indefinite(true));
    assert_eq!(definite, indefinite);
    assert_eq!(
        DataItem::from("ab"),
        DataItem::Text(TextString::from_chunks(["a", "b"]))
    );
}

#[test]
fn most_negative_integers_convert() {
    assert_eq!(
        DataItem::from(i64::MIN).as_integer(),
        Some(i128::from(i64::MIN))
    );
    assert_eq!(
        DataItem::try_from(-(1_i128 << 64)).unwrap(),
        DataItem::Signed(u64::MAX)
    );
    assert!(DataItem::try_from(i128::MIN).is_err());
}

#[test]
fn key_equivalence_follows_the_generic_data_model() {
    let duplicate = |hex_cbor: &str| {
        matches!(
            DataItem::decode(&hex_bytes(&hex_cbor.replace(' ', ""))),
            Err(Error::DuplicateMapKey(_))
        )
    };
    // strings and arrays match across length forms
    assert!(duplicate("a2 4101 02 5f4101ff 03"));
    // equivalence reaches into nested keys
    assert!(duplicate("a2 81f90000 01 81f98000 02"));
    assert!(duplicate("a2 c1f90000 01 c1f98000 02"));
    assert!(duplicate("a2 a2 0102 0304 01 a2 0304 0102 02"));
    assert!(duplicate("a2 8101 01 9f01ff 02"));
    assert!(duplicate("a2 a10101 01 bf0101ff 02"));
    // nested maps are checked too
    assert!(duplicate("a1 01 a2 02 03 02 04"));
    assert!(duplicate("bf 01 02 01 03 ff"));
    // different types are distinct
    assert!(!duplicate("a2 6161 01 4161 02"));
    assert!(!duplicate("a2 01 02 c101 03"));
    assert!(!duplicate("a2 02 01 e2 02"));
}

#[test]
fn lookup_finds_a_key_whatever_its_length_form() {
    use crate::index::Get as _;

    let item = DataItem::decode(&hex_bytes("a17f626865636c6c6fff01")).unwrap();
    assert_eq!(item.get("hello"), Some(&DataItem::from(1)));
}

#[test]
fn nan_significand_survives_every_width() {
    // a signaling NaN keeps its significand at every width
    let half = DataItem::decode(&hex_bytes("f97c01")).unwrap();
    let single = DataItem::decode(&hex_bytes("fa7f802000")).unwrap();
    let double = DataItem::decode(&hex_bytes("fb7ff0040000000000")).unwrap();
    assert_eq!(half, single);
    assert_eq!(half, double);
    // these are already at their shortest width
    for hex_cbor in [
        "f97c01",
        "f97e01",
        "f9fc01",
        "fa7f800001",
        "fa7fc00001",
        "fb7ff0000000000001",
        "fb7ff8000000000001",
    ] {
        let bytes = hex_bytes(hex_cbor);
        assert_eq!(
            DataItem::decode(&bytes).unwrap().encode(),
            bytes,
            "{hex_cbor}"
        );
    }
}

#[test]
fn sequence_delivers_items_before_a_failure() {
    let decoder = Decoder::new();
    let bytes = hex_bytes("010218");
    let items = decoder.sequence(&bytes).collect::<Vec<_>>();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0], Ok(DataItem::from(1)));
    assert_eq!(items[1], Ok(DataItem::from(2)));
    assert!(items[2].is_err());
    // an empty sequence has no items, and a break is not an item
    assert_eq!(decoder.sequence(&[]).count(), 0);
    assert_eq!(decoder.sequence(&[0x01, 0xff, 0x02]).count(), 2);
}
