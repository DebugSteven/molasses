// Suppose usize is u64. If there are k := 2^(63)+1 leaves, then there are a total of 2(k-1) + 1 =
// 2(2^(63))+1 = 2^(64)+1 nodes in the tree, which is outside the representable range. So our upper
// bound is 2^(63) leaves, which gives a tree with 2^(64)-1 nodes.
const MAX_LEAVES: usize = (std::usize::MAX >> 1) + 1;

/// Returns `Some(floor(log2(x))` when `x != 0`, and `None` otherwise
fn log2(x: usize) -> Option<usize> {
    // The log2 of x is the position of its most significant bit
    let bitlen = (0usize).leading_zeros() as usize;
    (bitlen - x.leading_zeros() as usize).checked_sub(1)
}

/// Computes the level of a given node in a binary left-balanced tree. Leaves are level 0, their
/// parents are level 1, etc. If a node's children are at different level, then its level is the
/// max level of its children plus one.
pub(crate) fn node_level(idx: usize) -> usize {
    // The level of idx is equal to the number of trialing 1s in its binary representation.
    // Equivalently, this is just the number of trailing zeros of (NOT idx)
    (!idx).trailing_zeros() as usize
}

/// Computes the number of nodes needed to represent a tree with `num_leaves` many leaves
///
/// Panics: when `num_leaves == 0` or `num_leaves > MAX_LEAVES`
fn num_nodes_in_tree(num_leaves: usize) -> usize {
    assert!(num_leaves > 0 && num_leaves <= MAX_LEAVES);
    2 * (num_leaves - 1) + 1
}

/// Computes the number of leaves in a tree of `num_nodes` many nodes
///
/// Panics: when `num_nodes` is odd, since all left-balanced binary trees have an odd number of
/// nodes
pub(crate) fn num_leaves_in_tree(num_nodes: usize) -> usize {
    assert!(num_nodes % 2 == 1);
    // Inverting the formula for num_nodes_in_tree, we get num_leaves = (num_nodes-1)/2 + 1
    ((num_nodes - 1) >> 1) + 1
}

/// Computes the index of the root node of a tree with `num_leaves` many leaves
///
/// Panics: when `num_leaves == 0` or `num_leaves > MAX_LEAVES`
fn root_idx(num_leaves: usize) -> usize {
    assert!(num_leaves > 0 && num_leaves <= MAX_LEAVES);
    // Root nodes are always index 2^n - 1 where n is the smallest number such that the size of the
    // tree is less than the next power of 2, i.e., 2^(n+1).
    let n = num_nodes_in_tree(num_leaves);
    (1 << log2(n).unwrap()) - 1
}

/// Computes the index of the left child of a given node. This does not depend on the size of the
/// tree. The child of a leaf is itself.
pub(crate) fn node_left_child(idx: usize) -> usize {
    let lvl = node_level(idx);
    // The child of a leaf is itself
    if lvl == 0 {
        idx
    } else {
        // Being on the n-th level (index 0) means your index is of the form xyz..01111...1 where
        // x,y,z are arbitrary, and there are n-many ones at the end. Stepping to the left is
        // equivalent to clearing the highest trailing 1.
        idx ^ (0x01 << (lvl - 1))
    }
}

/// Computes the index of the left child of the given node. The child of a leaf is itself.
///
/// Panics: when `num_leaves == 0` or `num_leaves > MAX_LEAVES` or
/// `idx >= num_nodes_in_tree(num_leaves)`
pub(crate) fn node_right_child(idx: usize, num_leaves: usize) -> usize {
    assert!(num_leaves > 0 && num_leaves <= MAX_LEAVES);
    assert!(idx < num_nodes_in_tree(num_leaves));

    let lvl = node_level(idx);
    // The child of a leaf is itself
    if lvl == 0 {
        idx
    } else {
        // Being on the n-th level (index 0) means your index is of the form xyz..01111...1 where
        // x,y,z are arbitrary, and there are n-many ones at the end. Stepping to the right is
        // equivalent to setting the rightmost 0 to a 1 and the highest trailing 1 to a 0. However,
        // this node might not exist (e.g., in a tree of 3 leaves, the right child of the root node
        // (idx 3) is the node with idx 4, not 5, since the rightmost tree isn't full). So we start
        // at the conjectured node and move left until we are within the bounds of the tree. This
        // is guaranteed to terminate, because if it didn't, there couldn't be any nodes with index
        // higher than the parent, which violates the invariant that every non-leaf node has two
        // children.
        let mut r = idx ^ (0x03 << (lvl - 1));
        let idx_threshold = num_nodes_in_tree(num_leaves);
        while r >= idx_threshold {
            r = node_left_child(r);
        }

        r
    }
}

/// Computes the index of the parent of a given node. The parent of the root is the root.
///
/// Panics: when `num_leaves == 0` or `num_leaves > MAX_LEAVES` or
/// `idx >= num_nodes_in_tree(num_leaves)`
fn node_parent(idx: usize, num_leaves: usize) -> usize {
    // The immediate parent of a node. May be beyond the right edge of the tree. This means weird
    // overflowing behavior when i == usize::MAX. However, this case is caught by the check below
    // that idx == root_idx(num_leaves). We hit the overflowing case iff idx is usize::MAX, which
    // is of the form 2^n - 1 for some n, which means that it's the root of a completely full tree
    // or it's the root of a subtree with more than `MAX_LEAVES` elements. The former case is
    // handled by the first if-statement below, and the latter is handled by the assert below.
    fn parent_step(i: usize) -> usize {
        // Recall that the children of xyz...0111...1 are xyz...0011...1 and xyz...1011...1 Working
        // backwards, this means that the parent of something that ends with 0011...1 or
        // 1011...1 is 0111...1. So if i is the index of the least significant 0, we must clear the
        // (i+1)-th bit and set the i-th bit.
        // This might be off the edge of the tree, since if, say, we have a tree on 3 leaves, the
        // rightmost leaf is idx 4, whose parent according to this algorithm would be idx 5, which
        // doesn't exist.
        let lvl = node_level(i);
        let bit_to_clear = i & (0x01 << (lvl + 1));
        let bit_to_set = 0x01 << lvl;

        (i | bit_to_set) ^ bit_to_clear
    }

    assert!(num_leaves > 0 && num_leaves <= MAX_LEAVES);
    assert!(idx < num_nodes_in_tree(num_leaves));

    if idx == root_idx(num_leaves) {
        idx
    } else {
        // First assume we're in a full tree. This means we're assuming the direct path of this
        // node is maximally long.
        let mut p = parent_step(idx);
        let idx_threshold = num_nodes_in_tree(num_leaves);
        // This must terminate, since stepping up will eventually land us at the root node of the
        // tree, and parent_step increases the level at every step. The algorithm is correct, since
        // the direct path of the node of index i ocurring in a non-full subtree is a subpath of
        // the node of index i ocurring in a full subtree. Since they share an ancestor, we'll
        // eventually reach it if we start from the bottom and work our way up.
        while p >= idx_threshold {
            p = parent_step(p);
        }

        p
    }
}

/// Computes the index of the sibling of a given node. The sibling of the root is the root.
///
/// Panics: when `num_leaves == 0` or `num_leaves > MAX_LEAVES` or
/// `idx >= num_nodes_in_tree(num_leaves)`
fn node_sibling(idx: usize, num_leaves: usize) -> usize {
    assert!(num_leaves > 0 && num_leaves <= MAX_LEAVES);
    assert!(idx < num_nodes_in_tree(num_leaves));

    // Recall that the left and right children of xyz...0111...1 are xyz...0011...1 and
    // xyz...1011...1, respectively. The former is less than the initial index, and the latter is
    // greater. So left is smaller, right is greater.
    let parent = node_parent(idx, num_leaves);
    if idx < parent {
        // We were on the left child, so return the right
        node_right_child(parent, num_leaves)
    } else if idx > parent {
        // We were on the right child, so return the left
        node_left_child(parent)
    } else {
        // We're at the root, so return the root
        parent
    }
}

// TODO: Consider making direct_path and copath into iterators so that we don't have to allocate

/// Returns the direct path of a given node in the form `[i_1, i_2, ..., i_n]` where
/// `i_1` is the parent of the given node and `i_n` is a child of the root node.
///
/// Panics: when `num_leaves == 0` or `num_leaves > MAX_LEAVES` or
/// `start_idx >= num_nodes_in_tree(num_leaves)`
fn node_direct_path(start_idx: usize, num_leaves: usize) -> Vec<usize> {
    assert!(num_leaves > 0 && num_leaves <= MAX_LEAVES);
    assert!(start_idx < num_nodes_in_tree(num_leaves));

    let mut path = Vec::new();
    let root = root_idx(num_leaves);
    let mut p = node_parent(start_idx, num_leaves);

    // Start on the parent of the given node. Recall that the parent of the root is itself, so if
    // we're at the root, we return the empty vector. Similarly, if we're the child of the root, we
    // still return the empty vector.
    while p != root {
        path.push(p);
        p = node_parent(p, num_leaves);
    }

    path
}

/// Returns the copath path of a given node in the form `[i_1, i_2, ..., i_n]` where
/// `i_1` is the sibling of the given node and `i_n` is a child of the root node.
///
/// Panics: when `num_leaves == 0` or `num_leaves > MAX_LEAVES` or
/// `start_idx >= num_nodes_in_tree(num_leaves)`
fn node_copath(start_idx: usize, num_leaves: usize) -> Vec<usize> {
    assert!(num_leaves > 0 && num_leaves <= MAX_LEAVES);
    assert!(start_idx < num_nodes_in_tree(num_leaves));

    let mut copath = Vec::new();
    let root = root_idx(num_leaves);
    let mut p = start_idx;

    // Iterate up the direct path starting at the given node, taking siblings along the way.
    while p != root {
        // Recall that p has no siblings iff it is the root node, so it's guaranteed that
        // sibling != p here.
        let sibling = node_sibling(p, num_leaves);
        copath.push(sibling);
        p = node_parent(p, num_leaves);
    }

    copath
}

/// Returns a list of root node indices for maximal subtrees of a tree of a given size
///
/// Panics: when `num_leaves == 0` or `num_leaves > MAX_LEAVES`
fn tree_frontier(num_leaves: usize) -> Vec<usize> {
    assert!(num_leaves > 0 && num_leaves <= MAX_LEAVES);

    // The given tree has a maximal subtree of size 2^(i+1)-1 exists iff the i-th bit (indexing at
    // 0) is set in the binary representation of num_leaves. We store the sizes by the number of
    // leaves in the tree, i.e., 2^i where i is as above.
    let mut sizes_present = Vec::new();
    for j in 0..=log2(num_leaves).unwrap() {
        let bitmask = 1 << j;
        if num_leaves & bitmask != 0 {
            sizes_present.push(bitmask);
        }
    }

    let mut base = 0;
    let mut frontier = Vec::new();
    // Iterate from largest to smallest subtrees, since the largest occur on the left.
    for num_leaves_in_subtree in sizes_present.into_iter().rev() {
        frontier.push(root_idx(num_leaves_in_subtree) + base);
        let num_nodes_in_subtree = num_nodes_in_tree(num_leaves_in_subtree);
        // Advance the index to the next relevant subtree. There's a +1 because we skip over the
        // parent of the maximal subtree we were just at.
        // Efficiency note: this is equivalent to writing base |= num_leaves_in_subtree << 1;
        base += num_nodes_in_subtree + 1;
    }

    frontier
}

/// Returns a list of indices for leaf nodes in a tree of given size
///
/// Panics: when `num_leaves == 0` or `num_leaves > MAX_LEAVES`
fn tree_leaves(num_leaves: usize) -> Vec<usize> {
    assert!(num_leaves > 0 && num_leaves <= MAX_LEAVES);
    // The leaves are just all the even indices
    (0..num_leaves).map(|i| 2 * i).collect()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::tls_de::TlsDeserializer;

    use quickcheck::TestResult;
    use quickcheck_macros::quickcheck;
    use rand::Rng;
    use serde::de::Deserialize;

    #[test]
    fn log2_kat() {
        assert_eq!(log2(0), None);
        assert_eq!(log2(1), Some(0));
        assert_eq!(log2(2), Some(1));
        assert_eq!(log2(3), Some(1));
        assert_eq!(log2(127), Some(6));
        assert_eq!(log2(128), Some(7));
        assert_eq!(log2(129), Some(7));
        assert_eq!(log2(255), Some(7));

        // Check log2(x/2) == log2(x/4) + 1 where x == 2^n for the biggest possible n for usize
        let bigboi = std::usize::MAX;
        assert_eq!(
            log2((bigboi >> 1) + 1),
            log2((bigboi >> 2) + 1).map(|i| i + 1)
        );
    }

    // We'll use this tree for known-answer tests
    //               7
    //         _____/ \
    //        /        |
    //       3         |
    //     /   \       |
    //    /     \      |
    //   1       5     |
    //  / \     / \    |
    // 0   2   4   6   8

    // See above tree for a diagram
    #[test]
    fn node_level_simple_kat() {
        assert_eq!(node_level(0), 0);
        assert_eq!(node_level(1), 1);
        assert_eq!(node_level(2), 0);
        assert_eq!(node_level(3), 2);
        assert_eq!(node_level(4), 0);
        assert_eq!(node_level(5), 1);
        assert_eq!(node_level(6), 0);
        assert_eq!(node_level(7), 3);
        assert_eq!(node_level(8), 0);
    }

    #[test]
    fn num_nodes_in_tree_kat() {
        assert_eq!(num_nodes_in_tree(1), 1);
        assert_eq!(num_nodes_in_tree(4), 7);
        assert_eq!(num_nodes_in_tree(5), 9);

        // For explanation, see comments by definition of MAX_LEAVES
        assert_eq!(num_nodes_in_tree(MAX_LEAVES), std::usize::MAX);
    }

    #[test]
    fn num_leaves_in_tree_kat() {
        assert_eq!(num_leaves_in_tree(1), 1);
        assert_eq!(num_leaves_in_tree(7), 4);
        assert_eq!(num_leaves_in_tree(9), 5);

        // For explanation, see comments by definition of MAX_LEAVES
        assert_eq!(num_leaves_in_tree(std::usize::MAX), MAX_LEAVES);
    }

    // num_leaves_in_tree and num_nodes_in_tree are inverses of each other
    #[quickcheck]
    fn counting_correctness(num_nodes: usize) -> TestResult {
        // num_leaves_in_tree only works on odd inputs, so throw out the even ones
        if num_nodes % 2 == 0 {
            return TestResult::discard();
        }

        let n = num_nodes_in_tree(num_leaves_in_tree(num_nodes));
        TestResult::from_bool(n == num_nodes)
    }

    // See above tree for a diagram
    #[test]
    fn tree_relations_kat() {
        let num_leaves = 5;

        // Test parent relations
        assert_eq!(node_parent(0, num_leaves), 1);
        assert_eq!(node_parent(2, num_leaves), 1);
        assert_eq!(node_parent(4, num_leaves), 5);
        assert_eq!(node_parent(6, num_leaves), 5);
        assert_eq!(node_parent(1, num_leaves), 3);
        assert_eq!(node_parent(5, num_leaves), 3);
        assert_eq!(node_parent(3, num_leaves), 7);
        assert_eq!(node_parent(8, num_leaves), 7);
        assert_eq!(node_parent(7, num_leaves), 7);

        // Test leaf child relations
        assert_eq!(node_left_child(0), 0);
        assert_eq!(node_right_child(0, num_leaves), 0);
        assert_eq!(node_left_child(2), 2);
        assert_eq!(node_right_child(2, num_leaves), 2);
        assert_eq!(node_left_child(4), 4);
        assert_eq!(node_right_child(4, num_leaves), 4);
        assert_eq!(node_left_child(6), 6);
        assert_eq!(node_right_child(6, num_leaves), 6);
        assert_eq!(node_left_child(8), 8);
        assert_eq!(node_right_child(8, num_leaves), 8);

        // Test the non-leaf left relations
        assert_eq!(node_left_child(7), 3);
        assert_eq!(node_left_child(3), 1);
        assert_eq!(node_left_child(1), 0);
        assert_eq!(node_left_child(5), 4);

        // Test the non-leaf right relations
        assert_eq!(node_right_child(7, num_leaves), 8);
        assert_eq!(node_right_child(3, num_leaves), 5);
        assert_eq!(node_right_child(1, num_leaves), 2);
        assert_eq!(node_right_child(5, num_leaves), 6);

        // Test sibling relations
        assert_eq!(node_sibling(0, num_leaves), 2);
        assert_eq!(node_sibling(2, num_leaves), 0);
        assert_eq!(node_sibling(4, num_leaves), 6);
        assert_eq!(node_sibling(6, num_leaves), 4);
        assert_eq!(node_sibling(1, num_leaves), 5);
        assert_eq!(node_sibling(5, num_leaves), 1);
        assert_eq!(node_sibling(8, num_leaves), 3);
        assert_eq!(node_sibling(3, num_leaves), 8);
        assert_eq!(node_sibling(7, num_leaves), 7);
    }

    #[quickcheck]
    fn tree_relations_correctness(num_leaves: usize) {
        if num_leaves == 0 || num_leaves > MAX_LEAVES {
            // This is an invalid input. Do nothing.
            return;
        }

        let num_nodes = num_nodes_in_tree(num_leaves);

        // This is our starting node
        let me: usize = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0, num_nodes)
        };
        let my_sibling = node_sibling(me, num_leaves);
        let my_parent = node_parent(my_sibling, num_leaves);

        assert_eq!(node_parent(me, num_leaves), my_parent);

        // Recall left_child < parent < right_child
        match me.cmp(&my_parent) {
            std::cmp::Ordering::Less => {
                // I am the left child of my parent
                assert_eq!(node_left_child(my_parent), me);
                assert_eq!(node_right_child(my_parent, num_leaves), my_sibling);
            }
            std::cmp::Ordering::Greater => {
                // I am the left child of my parent
                assert_eq!(node_left_child(my_parent), my_sibling);
                assert_eq!(node_right_child(my_parent, num_leaves), me);
            }
            std::cmp::Ordering::Equal => {
                // I am my own parent. I must be the root node
                assert_eq!(root_idx(num_leaves), me);
            }
        }

        let my_left_child = node_left_child(me);
        let my_right_child = node_right_child(me, num_leaves);

        if my_left_child == me {
            // I'm a leaf. Make sure both my children are me.
            assert_eq!(my_right_child, me);
        } else {
            // I'm not a leaf. Make sure that my children are distinct, that they are siblings, and
            // that I'm their parent
            assert_ne!(my_left_child, my_right_child);
            assert_eq!(node_sibling(my_left_child, num_leaves), my_right_child);
            assert_eq!(node_sibling(my_right_child, num_leaves), my_left_child);
            assert_eq!(node_parent(my_left_child, num_leaves), me);
            assert_eq!(node_parent(my_right_child, num_leaves), me);
        }
    }

    // TODO: Add Panic tests

    // The following test vector is from
    // https://github.com/mlswg/mls-implementations/tree/master/test_vectors
    //
    // File: tree_math.bin
    //
    // struct {
    //   uint32 root<0..2^32-1>;
    //   uint32 left<0..2^32-1>;
    //   uint32 right<0..2^32-1>;
    //   uint32 parent<0..2^32-1>;
    //   uint32 sibling<0..2^32-1>;
    // } TreeMathTestVectors;
    //
    // These vectors have the following meaning, where the tree relations are as defined in the
    // specification
    //
    // * root[i] is the index of the root of a tree with i+1 leaves
    // * The remaining vectors are all within the context of a tree with 255 leaves:
    //   * left[i] is the index of the left child of node i
    //   * right[i] is the index of the right child of node i
    //   * parent[i] is the index of the parent of node i
    //   * sibling[i] is the index of the sibling of node i

    #[derive(Deserialize)]
    struct TreeMathTestVectors {
        #[serde(rename = "root__bound_u32")]
        root: Vec<u32>,
        #[serde(rename = "left__bound_u32")]
        left: Vec<u32>,
        #[serde(rename = "right__bound_u32")]
        right: Vec<u32>,
        #[serde(rename = "parent__bound_u32")]
        parent: Vec<u32>,
        #[serde(rename = "sibling__bound_u32")]
        sibling: Vec<u32>,
    }

    #[test]
    fn deserialize_test_vec() {
        let mut f = std::fs::File::open("test_vectors/tree_math.bin").unwrap();
        let mut deserializer = TlsDeserializer::from_reader(&mut f);
        let test_vec = TreeMathTestVectors::deserialize(&mut deserializer).unwrap();

        let size = 255;
        let num_root_ops = test_vec.root.len();
        let num_left_ops = test_vec.left.len();
        let num_right_ops = test_vec.right.len();
        let num_parent_ops = test_vec.parent.len();
        let num_sibling_ops = test_vec.sibling.len();

        let mut root: Vec<u32> = (1..=num_root_ops).map(|i| root_idx(i) as u32).collect();
        let mut left: Vec<u32> = (0..num_left_ops)
            .map(|i| node_left_child(i) as u32)
            .collect();
        let mut right: Vec<u32> = (0..num_right_ops)
            .map(|i| node_right_child(i, size) as u32)
            .collect();
        let mut parent: Vec<u32> = (0..num_parent_ops)
            .map(|i| node_parent(i, size) as u32)
            .collect();
        let mut sibling: Vec<u32> = (0..num_sibling_ops)
            .map(|i| node_sibling(i, size) as u32)
            .collect();

        assert_eq!(root, test_vec.root);
        assert_eq!(left, test_vec.left);
        assert_eq!(right, test_vec.right);
        assert_eq!(parent, test_vec.parent);
        assert_eq!(sibling, test_vec.sibling);
    }
}
