
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

### Optimization 1: Zero-value optimized hash function.

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


### Optimization 2: Compress the storage for duplicated nodes.

In the classic construction of SMT: We store a node with its two children as `Node {left, right}`, but with the zero-value optimization, mostly intermediate nodes in the tree are duplicated, these nodes occupied spaces but provide no additional information of the tree, we want our tree only stores unique-value nodes.

The idea is simple: for a single leaf SMT, we only store the leaf itself. When inserting another new leave, the merging happens. Instead of inserting a parent at each height, we only store the parent node once, plus the merging height of the two branches from leaves.

The trick is that we can simulate the `Node {leaf, right}` structure and pretend that we stored all the ancestor nodes in each height of the tree if we can extract the merging order information from somewhere.


The key to this trick is the leaf’s `key`. Each key in the SMT can be seen as a path from the tree’s root to the leaf. With the path information, we can extract the merging order of hashes at each height, so when inserting a new leaf, we also store the leaf’s key in node, and when we need to merge two nodes, we extract the merging order from the key path:

We can calculate the merging height of two leaves by their key(or key path):

```Rust
fn common_height(key1, key2) {
    for i in 255..0 {
        if key1.get_bit(i) != key2.get_bit(i) {
            // common height
            return i;
        }
    }
    return 0;
}
```

The node structure is like `BranchNode { fork_height, key, node, sibling}`, we use node to represent all duplicated intermediate nodes, plus an additional field key to store the path information, with key, we can calculate the merging order of nodes between height `[node.fork_height, 255]`.


* fork_height is the height that the node is merged; for a leaf, it is 0.
* key is copied from one of the node’s children. for a leaf node, the key is the leaf’s key.
* node and sibling are like the left and right in the classic node structure. The difference is that the position of nodes depends on the merging height.

To get a left child of a node in height H:

1. check H-th bit of key
2. if it is 1 means the node is on the right at height H, so sibling is the left child
3. if it is 0 means the node is on the left, so sibling is the right child
```Rust
// get children at height
// return value is (left, right)
fn children(branch_node, height) {
    let is_rhs = branch_node.key.get_bit(height);
    if is_rhs {
        return (branch_node.sibling, branch_node.node)
    } else {
        return (branch_node.node, branch_node.sibling)
    }
}
```

To get a leaf by a key, we walk down the tree from root to bottom:

```Rust
fn get(key) {
    let node = root;
    // path order by height
    let path = BTreeMap::new();
    loop {
        let branch_node = match map.get(node) {
            Some(b) => b,
            None => break,
        }
        // common height may be lower than node.fork_height
        let height = max(common_height(key, node.key), node.fork_height);
        if height > node.fork_height {
            // node is sibling, end search
            path.push(heignt, node);
            break;
        }
        // node is parent
        // extract children position from branch
        let (left, right) = children(branch_node, height);
        // extract key positon
        let is_right = key.get_bit(height);
        if is_right {
            path.push(height, left);
            node = right;
        } else {
            path.push(height, right);
            node = left;
        }
    }
    return self.leaves[node];
}
```

We use a similar algorithm to extract merging height information for other operations: updating and generate merkle proof. 



