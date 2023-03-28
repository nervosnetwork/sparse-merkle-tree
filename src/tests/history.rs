use crate::{blake2b::Blake2bHasher, default_store::DefaultStore, tests::utils::SMT, H256};

#[test]
fn test_access_history() {
    fn to_h256(n: u8) -> H256 {
        [n; 32].into()
    }

    fn update_batch(tree: &mut SMT, kvs: Vec<(u8, u8)>) -> H256 {
        for (k, v) in kvs {
            tree.update(to_h256(k), to_h256(v)).expect("update");
        }
        *tree.root()
    }

    fn check_batch(store: DefaultStore<H256>, root: H256, kvs: Vec<(u8, u8)>) {
        let tree = SMT::new(root, store);
        for (i, (k, v)) in kvs.into_iter().enumerate() {
            assert_eq!(tree.get(&to_h256(k)).expect("key"), to_h256(v));
            // check merkle proof
            let proof = tree.merkle_proof(vec![to_h256(k)]).expect("proof");
            let valid = proof
                .verify::<Blake2bHasher>(&root, vec![(to_h256(k), to_h256(v))])
                .expect("verify");
            assert!(valid, "key {}", i);
        }
    }

    let v1 = vec![(1, 1), (2, 2), (3, 3)];
    let v2 = vec![(1, 2), (3, 0)];
    let v3 = vec![(4, 5), (1, 0), (2, 0)];

    let mut tree = SMT::default();
    // insert v1..v3 kvs
    let v1_root = update_batch(&mut tree, v1.clone());
    let v2_root = update_batch(&mut tree, v2.clone());
    let v3_root = update_batch(&mut tree, v3.clone());

    // test v1..v3 kvs
    let store = tree.take_store();
    // v1
    check_batch(store.clone(), v1_root, v1);
    check_batch(store.clone(), v1_root, vec![(4, 0), (5, 0)]);
    // v2
    check_batch(store.clone(), v2_root, v2);
    check_batch(store.clone(), v2_root, vec![(2, 2), (4, 0), (5, 0)]);
    // v3
    check_batch(store.clone(), v3_root, v3);
    check_batch(store, v3_root, vec![(3, 0), (5, 0)]);
}
