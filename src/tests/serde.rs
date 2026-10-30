use std::collections::BTreeMap;

use serde::de::IntoDeserializer;
use serde::de::value::Error as ValueError;
use serde::{Deserialize, Serialize};

use crate::content::{Simple, Tag};
use crate::data_item::DataItem;
use crate::serde::{
    Error, Tagged, TaggedWith, Undefined, from_data_item, from_slice, to_data_item, to_vec,
};
use crate::tests::hex_bytes;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Point {
    x: i32,
    y: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum Shape {
    Empty,
    Circle(u32),
    Rect(u32, u32),
    Named { name: String },
}

fn rejected<T>(result: &Result<T, Error>) -> bool
where
    T: std::fmt::Debug,
{
    matches!(result, Err(Error::Unsupported(_)))
}

fn bytes(hex_cbor: &str) -> Vec<u8> {
    hex_bytes(&hex_cbor.replace(' ', ""))
}

#[test]
fn values_round_trip() {
    let point = Point {
        x: -7,
        y: Some(300),
    };
    assert_eq!(
        from_slice::<Point>(&to_vec(&point).unwrap()).unwrap(),
        point
    );
    let point = Point { x: 1, y: None };
    assert_eq!(
        from_slice::<Point>(&to_vec(&point).unwrap()).unwrap(),
        point
    );
    for shape in [
        Shape::Empty,
        Shape::Circle(2),
        Shape::Rect(3, 4),
        Shape::Named { name: "n".into() },
    ] {
        assert_eq!(
            from_slice::<Shape>(&to_vec(&shape).unwrap()).unwrap(),
            shape
        );
    }
    let map = BTreeMap::from([("a".to_string(), 1_u8), ("b".to_string(), 2)]);
    assert_eq!(
        from_slice::<BTreeMap<String, u8>>(&to_vec(&map).unwrap()).unwrap(),
        map
    );
}

#[test]
fn encoding_is_the_shortest_definite_form() {
    assert_eq!(
        to_vec(&Point { x: 1, y: None }).unwrap(),
        bytes("a2 6178 01 6179 f6")
    );
    assert_eq!(to_vec(&Shape::Empty).unwrap(), bytes("65 456d707479"));
    assert_eq!(
        to_vec(&Shape::Circle(2)).unwrap(),
        bytes("a1 66 436972636c65 02")
    );
    assert_eq!(to_vec(&1.5_f32).unwrap(), bytes("f93e00"));
}

#[test]
fn indefinite_items_are_read_with_chunks_joined() {
    // (_ "he", "llo") and [_ 1, 2]
    assert_eq!(
        from_slice::<String>(&bytes("7f 62 6865 63 6c6c6f ff")).unwrap(),
        "hello"
    );
    assert_eq!(
        from_slice::<Vec<u8>>(&bytes("9f 01 02 ff")).unwrap(),
        [1, 2]
    );
}

#[test]
fn unsupported_data_is_rejected_not_altered() {
    // a tag, undefined and a simple value in place of another type
    assert!(rejected(&from_slice::<u8>(&bytes("c1 01"))));
    assert!(rejected(&from_slice::<Option<u8>>(&bytes("f7"))));
    assert!(rejected(&from_slice::<u8>(&bytes("f0"))));
    // a tag is refused even when nothing asked for it, as an unknown field
    assert!(from_slice::<Point>(&bytes("a3 6178 01 6179 f6 617a c1 01")).is_err());
    // a bignum wider than 128 bits, or with content which is not bytes
    assert!(rejected(&from_slice::<u128>(&bytes(
        "c2 51 01 00000000 00000000 00000000 00000000"
    ))));
    assert!(rejected(&from_slice::<u128>(&bytes("c2 01"))));
    // -1 - 2^127 is just below i128
    assert!(rejected(&from_slice::<i128>(&bytes(
        "c3 50 80000000 00000000 00000000 00000000"
    ))));
}

#[test]
fn integers_outside_64_bits_are_bignums() {
    let unsigned_max = bytes("c2 50 ffffffff ffffffff ffffffff ffffffff");
    assert_eq!(to_vec(&u128::MAX).unwrap(), unsigned_max);
    assert_eq!(from_slice::<u128>(&unsigned_max).unwrap(), u128::MAX);
    // -1 - (2^127 - 1)
    let signed_min = bytes("c3 50 7fffffff ffffffff ffffffff ffffffff");
    assert_eq!(to_vec(&i128::MIN).unwrap(), signed_min);
    assert_eq!(from_slice::<i128>(&signed_min).unwrap(), i128::MIN);
    // the first numbers which do not fit a major type 0 or 1 item
    let two_pow_64 = bytes("c2 49 01 00000000 00000000");
    assert_eq!(to_vec(&(1_u128 << 64)).unwrap(), two_pow_64);
    assert_eq!(to_vec(&(1_i128 << 64)).unwrap(), two_pow_64);
    assert_eq!(from_slice::<u128>(&two_pow_64).unwrap(), 1 << 64);
    assert_eq!(from_slice::<i128>(&two_pow_64).unwrap(), 1 << 64);
    let below_two_pow_64 = bytes("c3 49 01 00000000 00000000");
    assert_eq!(to_vec(&(-(1_i128 << 64) - 1)).unwrap(), below_two_pow_64);
    assert_eq!(
        from_slice::<i128>(&below_two_pow_64).unwrap(),
        -(1_i128 << 64) - 1
    );
    // and the last ones which do
    assert_eq!(
        to_data_item(&-(1_i128 << 64)).unwrap(),
        DataItem::Signed(u64::MAX)
    );
    assert_eq!(
        to_data_item(&i128::from(u64::MAX)).unwrap(),
        DataItem::Unsigned(u64::MAX)
    );
    // a bignum which fits 64 bits is read all the same, leading zeros too
    assert_eq!(from_slice::<u8>(&bytes("c2 42 0005")).unwrap(), 5);
    assert_eq!(from_slice::<i8>(&bytes("c3 41 00")).unwrap(), -1);
    assert_eq!(
        from_slice::<u64>(&bytes("c2 48 ffffffff ffffffff")).unwrap(),
        u64::MAX
    );
    assert_eq!(from_slice::<f64>(&bytes("c2 41 05")).unwrap(), 5.0);
}

#[test]
fn tags_go_through_wrappers() {
    let tagged = Tagged {
        number: 1,
        content: 10_u8,
    };
    assert_eq!(to_vec(&tagged).unwrap(), bytes("c1 0a"));
    assert_eq!(from_slice::<Tagged<u8>>(&bytes("c1 0a")).unwrap(), tagged);
    assert_eq!(
        from_slice::<TaggedWith<1, u8>>(&bytes("c1 0a")).unwrap(),
        TaggedWith(10)
    );
    assert!(from_slice::<TaggedWith<2, u8>>(&bytes("c1 0a")).is_err());
    // a tag is needed where one is asked for
    assert!(from_slice::<Tagged<u8>>(&bytes("0a")).is_err());
    // tags nest and wrap any value
    let nested = Tagged {
        number: 1_000,
        content: Tagged {
            number: 2,
            content: Point { x: 1, y: None },
        },
    };
    let encoded = to_vec(&nested).unwrap();
    assert_eq!(encoded, bytes("d9 03e8 c2 a2 6178 01 6179 f6"));
    assert_eq!(
        from_slice::<Tagged<Tagged<Point>>>(&encoded).unwrap(),
        nested
    );
    // a tag with any content is a Tag
    let tag = Tag::new(1, DataItem::from(vec![1, 2]));
    assert_eq!(to_data_item(&tag).unwrap(), DataItem::Tag(tag.clone()));
    assert_eq!(from_slice::<Tag>(&bytes("c1 82 01 02")).unwrap(), tag);
    // through another format a tag is its number and its content
    assert_eq!(serde_json::to_string(&tagged).unwrap(), "[1,10]");
    assert_eq!(
        serde_json::from_str::<Tagged<u8>>("[1,10]").unwrap(),
        tagged
    );
    assert!(serde_json::from_str::<Tagged<u8>>("[1]").is_err());
}

#[test]
fn undefined_and_simple_values_go_through_wrappers() {
    assert_eq!(to_vec(&Undefined).unwrap(), bytes("f7"));
    assert_eq!(from_slice::<Undefined>(&bytes("f7")).unwrap(), Undefined);
    assert!(from_slice::<Undefined>(&bytes("f6")).is_err());
    assert_eq!(serde_json::to_string(&Undefined).unwrap(), "null");
    assert_eq!(
        serde_json::from_str::<Undefined>("null").unwrap(),
        Undefined
    );

    let simple = Simple::try_from(16).unwrap();
    assert_eq!(to_vec(&simple).unwrap(), bytes("f0"));
    assert_eq!(from_slice::<Simple>(&bytes("f0")).unwrap(), simple);
    let simple = Simple::try_from(255).unwrap();
    assert_eq!(to_vec(&simple).unwrap(), bytes("f8 ff"));
    assert_eq!(from_slice::<Simple>(&bytes("f8 ff")).unwrap(), simple);
    assert!(from_slice::<Simple>(&bytes("f6")).is_err());
    assert!(from_slice::<Simple>(&bytes("10")).is_err());
    assert_eq!(serde_json::to_string(&simple).unwrap(), "255");
    assert_eq!(serde_json::from_str::<Simple>("255").unwrap(), simple);
    // a number which is not a simple value is refused
    assert!(serde_json::from_str::<Simple>("24").is_err());
    assert!(serde_json::from_str::<Simple>("256").is_err());
}

#[test]
fn any_key_is_accepted_in_a_map() {
    let integer_keys = BTreeMap::from([(1_u8, 2_u8)]);
    assert_eq!(to_vec(&integer_keys).unwrap(), bytes("a1 01 02"));
    assert_eq!(
        from_slice::<BTreeMap<u8, u8>>(&bytes("a1 01 02")).unwrap(),
        integer_keys
    );
    let array_keys = BTreeMap::from([((1_u8, 2_u8), 3_u8)]);
    assert_eq!(to_vec(&array_keys).unwrap(), bytes("a1 82 01 02 03"));
    assert_eq!(
        from_slice::<BTreeMap<(u8, u8), u8>>(&bytes("a1 82 01 02 03")).unwrap(),
        array_keys
    );
    // serde names a variant by its index as well as by its name
    assert_eq!(
        from_slice::<Shape>(&bytes("a1 01 02")).unwrap(),
        Shape::Circle(2)
    );
    // another format applies its own rules to keys
    assert_eq!(
        serde_json::to_string(&DataItem::from(vec![(1, 2)])).unwrap(),
        r#"{"1":2}"#
    );
}

#[test]
fn duplicate_keys_are_rejected_when_writing() {
    struct Twice;
    impl Serialize for Twice {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            use serde::ser::SerializeMap as _;
            let mut map = serializer.serialize_map(None)?;
            map.serialize_entry("a", &1)?;
            map.serialize_entry("a", &2)?;
            map.end()
        }
    }
    assert!(rejected(&to_data_item(&Twice)));
}

#[test]
fn trailing_and_malformed_bytes_are_cbor_errors() {
    assert!(matches!(
        from_slice::<u8>(&bytes("01 02")),
        Err(Error::Cbor(_))
    ));
    assert!(matches!(
        from_slice::<u8>(&bytes("18")),
        Err(Error::Cbor(_))
    ));
}

#[test]
fn a_data_item_round_trips_through_itself() {
    let items = [
        DataItem::tag(1, 1),
        DataItem::Undefined,
        DataItem::GenericSimple(Simple::try_from(16).unwrap()),
        // a bignum keeps its leading zero
        DataItem::decode(&bytes("c2 42 0005")).unwrap(),
        DataItem::map([
            (DataItem::from(1), DataItem::tag(2, DataItem::bytes([1]))),
            (DataItem::from("a"), DataItem::Undefined),
            (DataItem::Null, DataItem::from(vec![DataItem::tag(3, 0)])),
        ]),
        DataItem::decode(&bytes("fa 7f800001")).unwrap(),
    ];
    for item in items {
        assert_eq!(to_data_item(&item).unwrap(), item, "{item}");
        assert_eq!(from_data_item::<DataItem>(&item).unwrap(), item, "{item}");
    }
}

#[test]
fn a_data_item_goes_through_other_formats() {
    let item = DataItem::map([("a", DataItem::from(vec![1, 2])), ("b", DataItem::Null)]);
    let json = serde_json::to_string(&item).unwrap();
    assert_eq!(json, r#"{"a":[1,2],"b":null}"#);
    assert_eq!(serde_json::from_str::<DataItem>(&json).unwrap(), item);
    // text and byte strings stay different kinds
    assert_eq!(to_data_item(&"a").unwrap(), DataItem::from("a"));
    // what serde has no counterpart for falls back to a plain structure
    assert_eq!(
        serde_json::to_string(&DataItem::tag(1, 1)).unwrap(),
        "[1,1]"
    );
    assert_eq!(serde_json::to_string(&DataItem::Undefined).unwrap(), "null");
    assert_eq!(
        serde_json::to_string(&DataItem::GenericSimple(Simple::try_from(16).unwrap())).unwrap(),
        "16"
    );
}

#[test]
fn signaling_nan_keeps_its_significand() {
    let bits = 0x7f80_0001_u32;
    let item = to_data_item(&f32::from_bits(bits)).unwrap();
    assert_eq!(item, DataItem::decode(&bytes("fa 7f800001")).unwrap());
    // and when a data item is read from a format which hands over an f32
    let read = DataItem::deserialize(IntoDeserializer::<ValueError>::into_deserializer(
        f32::from_bits(bits),
    ))
    .unwrap();
    assert_eq!(read, item);
    assert_eq!(read.encode(), bytes("fa 7f800001"));
}
