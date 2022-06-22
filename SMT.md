
Note: This document is based on the original blog post [An optimized compacted sparse merkle tree](https://justjjy.com/An-optimized-compact-sparse-merkle-tree)

### Introduction

Sparse merkle tree is an advanced data structure used in the blockchain world. It can compress a large key-value map into short bytes represented merkle root and generates existence or non-existence proof for keys in the tree.


In this article, I describe a construction method and several optimizations, which together make the construction of the sparse merkle tree with the following attributes:

* No pre-calculated hash set
* Efficient existence / non-existence proof
* Efficient storage space

Before diving into details, please make sure you completely understood what the sparse merkle tree is. These articles would be helpful if you never heard sparse merkle tree:


* [Whats a sparse merkle tree](https://medium.com/@kelvinfichter/whats-a-sparse-merkle-tree-acda70aeb837)
* [Optimizing sparse Merkle trees](https://ethresear.ch/t/optimizing-sparse-merkle-trees/3751)

### Optimization 1: Zero-value optimized hash function

A sparse merkle tree contains a lot of zero values. That is the reason why we call it a ‘sparse’ merkle tree. By optimizing these zero values, we can save a lot of calculations.

We define the node merging function as following:

* if L == 0, return R
* if R == 0, return L
* otherwise return sha256(L, R)

Note, this is a simple explanation. In real implementation, we also remember and hash the count of continuously zero in `L` or `R`.

In a naive sparse merkle tree construction, we usually pre-calculate the hash set of the default SMT(a tree that all values are zeros). When we access the default nodes, instead of duplicated calculation, we fetch the result from the pre-calculated hash set. The drawback is we need to save the pre-calculated result somewhere before we can use it. It may be costly, especially when we want to use SMT in a blockchain contract.

With optimization 1, we do not need the pre-calculated hash-set. For a default SMT, all the intermediate nodes are zero values. Thus, we only need to calculate the hash for non-zero nodes and return zero for the default nodes.


There’s only one issue that remains. The optimized hash function produces the same value from different key-value pairs, for example:  
`merge(N, 0) == merge(0, N)`. 
This behavior opens a weak point for the SMT. An attacker may construct a collision of merkle root from a faked key-value map.

To fix this, we use the result of `hash(key | value)`  as a leaf’s hash, for examples: 
`merge(N, leaf_hash(N, 0)) == merge(0, leaf_hash(0, N))` 
the result is false because the `leaf_hash(N, 0)` is never equals to `leaf_hash(0, N)` if `N != 0`, the attacker can’t construct a collision attacking.

Additionally, we store `leaf_hash -> value`  in a map to keep the reference to the original value.

We can prove the security of this construction.

* Since the key is included in the leaf’s hash, and key is unique in SMT, no matter what the value is, the `leaf_hash(key, value)` is unique.
* Each node is either merged by two different hashes or merged by a hash with a zero-value. We already knew that all leaves have a unique hash, so their parent nodes also have a unique hash, and so on. The parent of these parent nodes also has a unique hash until the root of the tree.
* We got zero value merkle root if the tree is empty. Otherwise, the root is merged from its two children nodes. Any changes in the leaves will also change the root hash.

So we believe this construction is security because we can’t construct a collision of merkle root.


### Optimization 2: abstract storage mechanics

In the classic construction of SMT: We store a branch node with its two children as `Node {left, right}`. We have implemented branch node as the following data structure:
```Rust
pub struct BranchNode {
    pub left: MergeValue,
    pub right: MergeValue,
}
```
When `left` and `right` are both zero,  it's not needed to store this zero node. 

We still face problem: too many branch nodes in real world. Assuming we have only one leaf node in SMT, there are still 255 branch nodes.
They are all ancestors of the only leaf node. The amount of branch nodes can be easily reached to millions when the leaf nodes grows to above 10k.
It can consume a lot of memory. We realize that the databases or kv-store will be used in SMT proof. Therefore, we introduce a abstracted layer of storage mechanics, `trait Store` as following:
```Rust
/// Trait for customize backend storage
pub trait Store<V> {
    fn get_branch(&self, branch_key: &BranchKey) -> Result<Option<BranchNode>, Error>;
    fn get_leaf(&self, leaf_key: &H256) -> Result<Option<V>, Error>;
    fn insert_branch(&mut self, node_key: BranchKey, branch: BranchNode) -> Result<(), Error>;
    fn insert_leaf(&mut self, leaf_key: H256, leaf: V) -> Result<(), Error>;
    fn remove_branch(&mut self, node_key: &BranchKey) -> Result<(), Error>;
    fn remove_leaf(&mut self, leaf_key: &H256) -> Result<(), Error>;
}
```

If branch nodes are not many, we can put them all in memory. By default, we have such `DefaultStore`:
```Rust
pub struct DefaultStore<V> {
    branches_map: Map<BranchKey, BranchNode>,
    leaves_map: Map<H256, V>,
}
```

If branch nodes are too many, it's better to implement a database backend. We suggest [RocksDB](http://rocksdb.org/): an embedded
persistent key-value store for fast storage. 

This store mechanics is only used in making SMT proof. It doesn't affect on-chain SMT verification which is still very fast and memory-efficient.
