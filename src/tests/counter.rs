use crate::branch::{BranchKey, BranchNode};
use crate::default_store::DefaultStore;
use crate::merge::MergeValue;
use crate::traits::StoreWriteOps;
use crate::H256;
#[test]
fn test_store_write_counter() {
    let mut store = DefaultStore::<u8>::default();
    store.enable_counter(true);
    assert!(store.is_counter_on());
    let random_key: H256 = H256::default();
    // single counts
    let _ = store.insert_leaf(random_key, 1);
    assert_eq!(store.leaves_counter().insert(), 1 as usize);
    let _ = store.remove_leaf(&random_key);
    assert_eq!(store.leaves_counter().remove(), 1 as usize);

    let branch_key = BranchKey {
        height: 1,
        node_key: random_key,
    };
    let branch_node = BranchNode {
        left: MergeValue::from_h256(random_key),
        right: MergeValue::from_h256(H256::zero()),
    };

    let _ = store.insert_branch(branch_key.clone(), branch_node);
    assert_eq!(store.branches_counter().insert(), 1 as usize);

    let _ = store.remove_branch(&branch_key);
    assert_eq!(store.branches_counter().remove(), 1 as usize);

    // resetting
    store.reset_all_counters();
    assert_eq!(store.leaves_counter().insert(), 0 as usize);
    assert_eq!(store.leaves_counter().remove(), 0 as usize);
    assert_eq!(store.branches_counter().insert(), 0 as usize);
    assert_eq!(store.branches_counter().remove(), 0 as usize);

    // looping
    for _ in 0..1000 {
        let _ = store.insert_leaf(random_key, 1);
    }
    assert_eq!(store.leaves_counter().insert(), 1000 as usize);

    for _ in 0..1000 {
        let _ = store.remove_leaf(&random_key);
    }
    assert_eq!(store.leaves_counter().remove(), 1000 as usize);

    for _ in 0..1000 {
        let branch_node = BranchNode {
            left: MergeValue::from_h256(random_key),
            right: MergeValue::from_h256(H256::zero()),
        };
        let _ = store.insert_branch(branch_key.clone(), branch_node);
    }
    assert_eq!(store.branches_counter().insert(), 1000 as usize);

    for _ in 0..1000 {
        let _ = store.remove_branch(&branch_key);
    }
    assert_eq!(store.branches_counter().remove(), 1000 as usize);

    // resetting
    store.reset_all_counters();
    assert_eq!(store.leaves_counter().insert(), 0 as usize);
    assert_eq!(store.leaves_counter().remove(), 0 as usize);
    assert_eq!(store.branches_counter().insert(), 0 as usize);
    assert_eq!(store.branches_counter().remove(), 0 as usize);
}
