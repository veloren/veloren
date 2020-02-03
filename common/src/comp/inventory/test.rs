use super::*;
use lazy_static::lazy_static;
lazy_static! {
    static ref TEST_ITEMS: Vec<Item> = vec![
        assets::load_expect_cloned("common.items.debug.boost"),
        assets::load_expect_cloned("common.items.debug.possess")
    ];
}
/// The `Default` inventory should contain two items
#[test]
fn create_default_count() { assert_eq!(Inventory::default().count(), 2) }

/// Attempting to push into a full inventory should return the same item.
#[test]
fn push_full() {
    let mut inv = Inventory {
        slots: TEST_ITEMS.iter().map(|a| Some(a.clone())).collect(),
    };
    assert_eq!(
        inv.push(TEST_ITEMS[0].clone()).unwrap(),
        TEST_ITEMS[0].clone()
    )
}

/// Attempting to push a series into a full inventory should return them all.
#[test]
fn push_all_full() {
    let mut inv = Inventory {
        slots: TEST_ITEMS.iter().map(|a| Some(a.clone())).collect(),
    };
    let Error::Full(leftovers) = inv
        .push_all(TEST_ITEMS.iter().map(|a| a.clone()))
        .expect_err("Pushing into a full inventory somehow worked!");
    assert_eq!(leftovers, TEST_ITEMS.clone())
}

/// Attempting to push uniquely into an inventory containing all the items
/// should work fine.
#[test]
fn push_unique_all_full() {
    let mut inv = Inventory {
        slots: TEST_ITEMS.iter().map(|a| Some(a.clone())).collect(),
    };
    inv.push_all_unique(TEST_ITEMS.iter().map(|a| a.clone()))
        .expect("Pushing unique items into an inventory that already contains them didn't work!");
}

/// Attempting to push uniquely into an inventory containing all the items
/// should work fine.
#[test]
fn push_all_empty() {
    let mut inv = Inventory {
        slots: vec![None, None],
    };
    inv.push_all(TEST_ITEMS.iter().map(|a| a.clone()))
        .expect("Pushing items into an empty inventory didn't work!");
}

/// Attempting to push uniquely into an inventory containing all the items
/// should work fine.
#[test]
fn push_all_unique_empty() {
    let mut inv = Inventory {
        slots: vec![None, None],
    };
    inv.push_all_unique(TEST_ITEMS.iter().map(|a| a.clone()))
        .expect(
            "Pushing unique items into an empty inventory that didn't contain them didn't work!",
        );
}
