use rand::seq::SliceRandom as _;

use crate::content::{Array, TextString};
use crate::data_item::DataItem;
use crate::deterministic::DeterministicMode;
use crate::encode::Encoder;

/// Keys of the test map along with the bytes each one encodes to
///
/// | key     | encoded  |
/// | ------- | -------- |
/// | `10`    | `0a`     |
/// | `100`   | `1864`   |
/// | `-1`    | `20`     |
/// | `"z"`   | `617a`   |
/// | `"aa"`  | `626161` |
/// | `[100]` | `811864` |
/// | `[-1]`  | `8120`   |
/// | `false` | `f4`     |
fn key(index: usize) -> DataItem {
    let keys = [
        DataItem::from(10),
        DataItem::from(100),
        DataItem::from(-1),
        DataItem::from("z"),
        DataItem::from("aa"),
        DataItem::from(vec![100]),
        DataItem::from(vec![DataItem::from(-1)]),
        DataItem::from(false),
    ];
    keys[index].clone()
}

/// The test map with its keys in the provided order
fn map_with_order(order: [usize; 8]) -> DataItem {
    DataItem::map(order.map(|index| (key(index), DataItem::from(u64::try_from(index).unwrap()))))
}

/// Order the keys take in each mode, worked out from the encoded keys above
fn expected_order(mode: DeterministicMode) -> [usize; 8] {
    match mode {
        // sorted by the encoded bytes
        DeterministicMode::Core => [0, 1, 2, 3, 4, 5, 6, 7],
        // sorted by encoded length first, then by the encoded bytes
        _ => [0, 2, 7, 1, 3, 6, 4, 5],
    }
}

fn check_mode(mode: DeterministicMode) {
    let sorted = map_with_order(expected_order(mode));
    assert!(sorted.is_deterministic(mode));

    let mut shuffled_order = expected_order(mode);
    while shuffled_order == expected_order(mode) {
        shuffled_order.shuffle(&mut rand::rng());
    }
    let shuffled = map_with_order(shuffled_order);
    assert!(!shuffled.is_deterministic(mode));

    // an encoder writes the deterministic form without touching the value
    assert_eq!(
        Encoder::new().deterministic(mode).encode(&shuffled),
        sorted.encode()
    );

    // and rewriting the value gives an item which is already deterministic
    let canonical = shuffled.into_deterministic(mode);
    assert!(canonical.is_deterministic(mode));
    assert_eq!(canonical.encode(), sorted.encode());
}

#[test]
fn core_mode() {
    check_mode(DeterministicMode::Core);
}

#[test]
fn length_first_mode() {
    check_mode(DeterministicMode::LengthFirst);
}

#[test]
fn modes_disagree_on_order() {
    // the key `100` encodes to `1864` and the key `false` to `f4`, so byte
    // ordering puts `100` first while length ordering puts `false` first
    let item = DataItem::map([(DataItem::from(100), "a"), (DataItem::from(false), "b")]);
    assert!(item.is_deterministic(DeterministicMode::Core));
    assert!(!item.is_deterministic(DeterministicMode::LengthFirst));
    assert_eq!(
        Encoder::new()
            .deterministic(DeterministicMode::LengthFirst)
            .encode(&item),
        DataItem::map([(DataItem::from(false), "b"), (DataItem::from(100), "a")]).encode()
    );
}

#[test]
fn sorts_nested_maps() {
    let item = DataItem::map([("outer", DataItem::map([(2, "b"), (1, "a")]))]);
    assert!(!item.is_deterministic(DeterministicMode::Core));
    assert_eq!(
        Encoder::new()
            .deterministic(DeterministicMode::Core)
            .encode(&item),
        DataItem::map([("outer", DataItem::map([(1, "a"), (2, "b")]))]).encode()
    );
}

#[test]
fn checks_nested_map_entries() {
    for mode in [DeterministicMode::Core, DeterministicMode::LengthFirst] {
        let indefinite_value = DataItem::map([(
            DataItem::from(1),
            DataItem::Array(Array::from_items([2]).with_indefinite(true)),
        )]);
        assert!(!indefinite_value.is_deterministic(mode));
        let canonical = indefinite_value.into_deterministic(mode);
        assert!(canonical.is_deterministic(mode));
        assert_eq!(canonical, DataItem::map([(1, vec![2])]));

        let indefinite_key = DataItem::map([(
            DataItem::Text(TextString::from_chunks(["a"])),
            DataItem::from(1),
        )]);
        assert!(!indefinite_key.is_deterministic(mode));
        let canonical = indefinite_key.into_deterministic(mode);
        assert!(canonical.is_deterministic(mode));
        assert_eq!(canonical, DataItem::map([("a", 1)]));
    }
}

#[test]
fn indefinite_items_are_not_deterministic() {
    for mode in [DeterministicMode::Core, DeterministicMode::LengthFirst] {
        let indefinite = [
            DataItem::Array(Array::from_items([1]).with_indefinite(true)),
            DataItem::Text(TextString::from_chunks(["a", "b"])),
            DataItem::bytes(crate::content::ByteString::from_chunks([vec![1]])),
        ];
        for item in indefinite {
            assert!(!item.is_deterministic(mode));
            assert!(item.into_deterministic(mode).is_deterministic(mode));
        }
    }
}

#[test]
fn key_lookup_still_works_after_reordering() {
    let shuffled = map_with_order([7, 6, 5, 4, 3, 2, 1, 0]);
    let canonical = shuffled.clone().into_deterministic(DeterministicMode::Core);
    let shuffled_entries = shuffled.as_map().unwrap().entries();
    let canonical_entries = canonical.as_map().unwrap().entries();
    assert_eq!(shuffled_entries.len(), canonical_entries.len());
    // an index map compares without regard to order, so the values must all
    // have travelled with their keys
    assert_eq!(shuffled_entries, canonical_entries);
    assert_eq!(canonical_entries.len(), 8);
}
