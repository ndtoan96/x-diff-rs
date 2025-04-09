use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use crate::tree::{XNode, XTree};
use md5::Digest;

trait Concat {
    fn concat(self, other: Self) -> Self;
}

impl Concat for Digest {
    fn concat(mut self, other: Self) -> Self {
        for i in 0..self.0.len() {
            self.0[i] = self.0[i].wrapping_add(other.0[i]);
        }
        self
    }
}

#[derive(Debug, Clone)]
pub enum Edit<'a, 'tree1, 'tree2> {
    Insert {
        child_node: XNode<'a, 'tree2>,
        to_node: XNode<'a, 'tree1>,
    },
    Delete(XNode<'a, 'tree1>),
    Update {
        old: XNode<'a, 'tree1>,
        new: XNode<'a, 'tree2>,
    },
    ReplaceRoot,
}

impl Display for Edit<'_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Edit::Insert {
                child_node,
                to_node,
            } => {
                write!(
                    f,
                    "insert node {} to node {}",
                    child_node.id(),
                    to_node.id()
                )
            }
            Edit::Delete(node) => write!(f, "delete node {}", node.id()),
            Edit::Update { old, new } => write!(
                f,
                "update node {}: {:?} -> {:?}",
                old.id(),
                old.value().unwrap().trim(),
                new.value().unwrap().trim()
            ),
            Edit::ReplaceRoot => write!(f, "replace root node"),
        }
    }
}

type Diff<'a, 'tree1, 'tree2> = Vec<Edit<'a, 'tree1, 'tree2>>;

/// Calculate the difference between two XML trees, represented by the minum edit operations to transform `tree1` to `tree2`.
pub fn diff<'a, 'doc1, 'doc2>(
    tree1: &'doc1 XTree<'doc1>,
    tree2: &'doc2 XTree<'doc2>,
) -> Diff<'a, 'doc1, 'doc2> {
    fn diff_node<'a, 'doc1, 'doc2>(
        node1: XNode<'a, 'doc1>,
        ht1: &HashMap<String, Digest>,
        node2: XNode<'a, 'doc2>,
        ht2: &HashMap<String, Digest>,
    ) -> Diff<'a, 'doc1, 'doc2> {
        if ht1.get(&node1.id().to_string()) == ht2.get(&node2.id().to_string()) {
            return Vec::new();
        }

        // Leaf nodes with different hashes mean different values
        if (node1.is_attribute() && node2.is_attribute()) || (node1.is_text() && node2.is_text()) {
            return vec![Edit::Update {
                old: node1,
                new: node2,
            }];
        }

        let mut iht1: HashMap<_, _> = node1
            .children()
            .iter()
            .map(|n| (*ht1.get(&n.id().to_string()).unwrap(), *n))
            .collect();
        let mut iht2: HashMap<_, _> = node2
            .children()
            .iter()
            .map(|n| (*ht2.get(&n.id().to_string()).unwrap(), *n))
            .collect();
        let children_hashes1: HashSet<_> = iht1.keys().copied().collect();
        let children_hashes2: HashSet<_> = iht2.keys().copied().collect();
        let same_hashes: HashSet<_> = children_hashes1.intersection(&children_hashes2).collect();
        iht1.retain(|k, _| !same_hashes.contains(&k));
        iht2.retain(|k, _| !same_hashes.contains(&k));
        let mut remaining_children1: HashSet<_> = iht1.into_values().collect();
        let mut remaining_children2: HashSet<_> = iht2.into_values().collect();
        let mut diff_pairs = Vec::new();
        for n1 in &remaining_children1 {
            for n2 in &remaining_children2 {
                if n1.signature() == n2.signature() {
                    diff_pairs.push((*n1, *n2, diff_node(*n1, ht1, *n2, ht2)));
                }
            }
        }
        diff_pairs.sort_by_key(|item| item.2.len());
        let mut diff = Vec::new();
        for (n1, n2, mut d) in diff_pairs {
            if remaining_children1.contains(&n1) && remaining_children2.contains(&n2) {
                diff.append(&mut d);
                remaining_children1.remove(&n1);
                remaining_children2.remove(&n2);
            }
        }
        for n1 in remaining_children1 {
            diff.push(Edit::Delete(n1));
        }
        for n2 in remaining_children2 {
            diff.push(Edit::Insert {
                child_node: n2,
                to_node: node1,
            });
        }
        diff
    }
    if tree1.root().signature() != tree2.root().signature() {
        return vec![Edit::ReplaceRoot];
    }
    let ht1 = calculate_hash_table(tree1);
    let ht2 = calculate_hash_table(tree2);
    diff_node(tree1.root(), &ht1, tree2.root(), &ht2)
}

fn calculate_hash_table(tree: &XTree) -> HashMap<String, Digest> {
    fn hash_of_node(node: XNode, ht: &mut HashMap<String, Digest>) -> Digest {
        let hash = if node.children().is_empty() {
            node.hash()
        } else {
            let mut acc = node.hash();
            for child in node.children() {
                acc = acc.concat(hash_of_node(child, ht));
            }
            acc
        };
        ht.insert(node.id().to_string(), hash);
        hash
    }
    let mut hash_table = HashMap::new();
    hash_of_node(tree.root(), &mut hash_table);
    hash_table
}

#[cfg(test)]
mod test {
    #[cfg(feature = "print")]
    use crate::tree::print::{PrintTreeOptions, print_tree};

    use super::*;
    use std::fs;

    #[test]
    fn test_calculate_hash_table_same_tree() {
        let text1 = fs::read_to_string("test/file1.xml").unwrap();
        let tree1 = XTree::parse(&text1).unwrap();
        let ht1 = calculate_hash_table(&tree1);

        let text2 = fs::read_to_string("test/file1.xml").unwrap();
        let tree2 = XTree::parse(&text2).unwrap();
        let ht2 = calculate_hash_table(&tree2);

        assert_eq!(
            ht1.get(&tree1.root().id().to_string()),
            ht2.get(&tree2.root().id().to_string())
        );
    }

    #[test]
    fn test_calculate_hash_table_different_tree() {
        let text1 = fs::read_to_string("test/file1.xml").unwrap();
        let tree1 = XTree::parse(&text1).unwrap();
        let ht1 = calculate_hash_table(&tree1);

        let text2 = fs::read_to_string("test/file2.xml").unwrap();
        let tree2 = XTree::parse(&text2).unwrap();
        let ht2 = calculate_hash_table(&tree2);

        assert_ne!(
            ht1.get(&tree1.root().id().to_string()),
            ht2.get(&tree2.root().id().to_string())
        );
    }

    #[test]
    fn test_diff() {
        let text1 = fs::read_to_string("test/file1.xml").unwrap();
        let tree1 = XTree::parse(&text1).unwrap();

        let text2 = fs::read_to_string("test/file2.xml").unwrap();
        let tree2 = XTree::parse(&text2).unwrap();

        #[cfg(feature = "print")]
        {
            print_tree(&tree1, PrintTreeOptions::default().with_node_id());
            print_tree(&tree2, PrintTreeOptions::default().with_node_id());
        }

        let diff = diff(&tree1, &tree2);
        diff.iter().any(|d| {
            regex::Regex::new(r#"update node \d+: \"George\" -> \"Fred\""#)
                .unwrap()
                .is_match(&d.to_string())
        });
        diff.iter().any(|d| {
            regex::Regex::new(r#"insert node \d+[Attr] to node \d+"#)
                .unwrap()
                .is_match(&d.to_string())
        });
        diff.iter().any(|d| {
            regex::Regex::new(r#"insert node \d+[Foo] to node \d+"#)
                .unwrap()
                .is_match(&d.to_string())
        });
        for e in diff {
            println!("{}", e);
        }
    }
}
