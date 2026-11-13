use sparse_merkle_tree::{
    blake2b::Blake2bHasher, default_store::DefaultStore, error::Error, CompiledMerkleProof,
    MerkleProof, SparseMerkleTree, H256,
};

type SMT = SparseMerkleTree<Blake2bHasher, H256, DefaultStore<H256>>;

fn key(low_byte: u8) -> H256 {
    let mut bytes = [0u8; 32];
    bytes[0] = low_byte;
    bytes.into()
}

#[test]
fn rejects_same_side_merge_at_height_one() {
    // Both pairs fork at height 0 but have the same bit at height 1.
    for (left, right) in [(key(0), key(1)), (key(2), key(3))] {
        let value_left = [0xaa; 32].into();
        let value_right = [0xbb; 32].into();
        let leaves = vec![(left, value_left), (right, value_right)];
        let mut tree = SMT::default();
        tree.update(left, value_left).unwrap();
        tree.update(right, value_right).unwrap();

        let valid = tree
            .merkle_proof(vec![left, right])
            .unwrap()
            .compile(vec![left, right])
            .unwrap();
        assert_eq!(
            valid.compute_root::<Blake2bHasher>(leaves.clone()),
            Ok(*tree.root())
        );

        let malformed = CompiledMerkleProof(vec![
            0x4c, 0x4f, 0x01, // first leaf, then climb to height 1
            0x4c, 0x4f, 0x01, // second leaf, then climb to height 1
            0x48, // merge two nodes on the same side
            0x4f, 0xfe, // climb to root
        ]);
        assert_eq!(left.parent_path(1), right.parent_path(1));
        assert_eq!(left.get_bit(1), right.get_bit(1));
        assert_eq!(
            malformed.compute_root::<Blake2bHasher>(leaves.clone()),
            Err(Error::CorruptedProof)
        );
        assert_eq!(
            malformed.verify::<Blake2bHasher>(tree.root(), leaves),
            Err(Error::CorruptedProof)
        );
    }
}

#[test]
fn accepts_opposite_side_merge_at_height_one() {
    let left = key(0);
    let right = key(2);
    let value_left = [0xaa; 32].into();
    let value_right = [0xbb; 32].into();
    let mut tree = SMT::default();
    tree.update(left, value_left).unwrap();
    tree.update(right, value_right).unwrap();

    let proof = CompiledMerkleProof(vec![
        0x4c, 0x4f, 0x01, // first leaf, then climb to height 1
        0x4c, 0x4f, 0x01, // second leaf, then climb to height 1
        0x48, // merge opposite sides at height 1
        0x4f, 0xfe, // climb to root
    ]);
    assert_ne!(left.get_bit(1), right.get_bit(1));
    assert_eq!(
        proof.compute_root::<Blake2bHasher>(vec![(left, value_left), (right, value_right)]),
        Ok(*tree.root())
    );
}

#[test]
fn rejects_duplicate_leaf_keys() {
    let leaf = (key(0), [0xaa; 32].into());
    let leaves = vec![leaf, leaf];
    let malformed = CompiledMerkleProof(vec![
        0x4c, 0x4c, 0x48, // merge the same key with itself at height 0
        0x4f, 0xff, // climb to root
    ]);
    assert_eq!(
        malformed.compute_root::<Blake2bHasher>(leaves),
        Err(Error::CorruptedProof)
    );
}

#[test]
fn refuses_to_compile_duplicate_leaf_keys() {
    let key = key(0);
    assert_eq!(
        MerkleProof::new(vec![H256::zero(); 2], vec![])
            .compile(vec![key; 2])
            .unwrap_err(),
        Error::CorruptedProof
    );
}
