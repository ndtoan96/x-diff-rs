use std::collections::{HashMap, HashSet};

use md5::Digest;
use tree::{XNode, XNodeId, XTree};

pub mod tree;

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

// pub enum Edit<'tree1, 'tree2> {
//     Insert(XNodeId<'tree2>),
//     Delete(XNodeId<'tree1>),
//     Update {
//         node_id: XNodeId<'tree1>,
//         old_value: &'tree1 str,
//         new_value: &'tree2 str,
//     },
// }

// pub fn diff<'tree1, 'tree2>(
//     tree1: &'tree1 XTree,
//     tree2: &'tree2 XTree,
// ) -> Vec<Edit<'tree1, 'tree2>> {
//     todo!()
// }

fn calculate_hash_table<'doc>(tree: &'doc XTree) -> HashMap<XNodeId<'doc>, Digest> {
    fn hash_of_node<'a, 'doc>(
        node: &'a XNode<'a, 'doc>,
        ht: &mut HashMap<XNodeId<'doc>, Digest>,
    ) -> Digest {
        let hash = if node.children().len() == 0 {
            node.hash()
        } else {
            let mut acc = node.hash();
            for child in node.children() {
                acc = acc.concat(hash_of_node(&child, ht));
            }
            acc
        };
        ht.insert(node.id(), hash);
        return hash;
    }
    let mut hash_table = HashMap::new();
    hash_of_node(&tree.root(), &mut hash_table);
    hash_table
}

#[cfg(test)]
mod test {
    use std::fs;
    use tree::XTreePrintOptions;

    use super::*;

    #[test]
    fn test_calculate_hash_table_same_tree() {
        let text1 = fs::read_to_string("file1.xml").unwrap();
        let tree1 = XTree::parse(&text1).unwrap();
        let ht1 = calculate_hash_table(&tree1);

        let text2 = fs::read_to_string("file1.xml").unwrap();
        let tree2 = XTree::parse(&text2).unwrap();
        let ht2 = calculate_hash_table(&tree2);

        assert_eq!(ht1.get(&tree1.root().id()), ht2.get(&tree2.root().id()));
    }

    #[test]
    fn test_calculate_hash_table_different_tree() {
        let text1 = fs::read_to_string("file1.xml").unwrap();
        let tree1 = XTree::parse(&text1).unwrap();
        let ht1 = calculate_hash_table(&tree1);
        let hex_marker1 = ht1
            .iter()
            .map(|(k, v)| (*k, format!("{} - {:x}", k, v)))
            .collect();
        tree1.print(XTreePrintOptions::default().with_node_marker(&hex_marker1));

        let text2 = fs::read_to_string("file2.xml").unwrap();
        let tree2 = XTree::parse(&text2).unwrap();
        let ht2 = calculate_hash_table(&tree2);
        let hex_marker2 = ht2
            .iter()
            .map(|(k, v)| (*k, format!("{} - {:x}", k, v)))
            .collect();
        tree2.print(XTreePrintOptions::default().with_node_marker(&hex_marker2));

        assert_ne!(ht1.get(&tree1.root().id()), ht2.get(&tree2.root().id()));
    }
}
