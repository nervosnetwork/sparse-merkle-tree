# Sparse merkle tree

[![Crates.io](https://img.shields.io/crates/v/sparse-merkle-tree.svg)](https://crates.io/crates/sparse-merkle-tree)
[Docs](https://docs.rs/sparse-merkle-tree)

An optimized sparse merkle tree.

| size | proof size | update | get | merkle proof | verify proof |
| --- | --- | --- | --- | --- | --- |
| 2n + log(n) | log(n) | log(n) | log(n) | log(n) | log(n) |

Features:

* Multi-leaves existence / non-existence merkle proof
* Customizable hash function
* Rust `no_std` support

This article describes algorithm of this data structure [An optimized compacted sparse merkle tree](https://justjjy.com/An-optimized-compact-sparse-merkle-tree)

**Notice** this library is not stabled yet. The API and the format of the proof may be changed in the future. Make sure you know what you are doing before using this library.

## Construction

A sparse merkle tree is a perfectly balanced tree contains `2 ^ N` leaves:

``` txt
# N = 256 sparse merkle tree
height:
255                0
                /     \
254            0        1

.............................

           /   \          /  \
1         0     1        0    1
         / \   / \      / \   / \
0       0   1 0  1 ... 0   1 0   1 
       0x00..00 0x00..01   ...   0x11..11
```

The above graph demonstrates a sparse merkle tree with `2 ^ 256` leaves, which can mapping every possible `H256` value into leaves. The height of the tree is `256`, from top to bottom, we denote `0` for each left branch and denote `1` for each right branch, so we can get a 256 bits path, which also can represent in `H256`, we use the path as the key of leaves, the most left leaf's key is `0x00..00`, and the next key is `0x00..01`, the most right key is `0x11..11`.

## License

MIT
