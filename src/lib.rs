use std::collections::{HashMap, HashSet};

use md5::Digest;
use tree::{XNodeId, XTree};

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
    let mut hash_table = HashMap::new();
    let mut parents = HashSet::new();
    for node in tree.get_leaves_nodes() {
        hash_table.insert(node.id(), node.hash());
        if let Some(parent) = node.parent() {
            parents.insert(parent);
        }
    }
    while parents.len() > 0 {
        let mut tmp_parents = HashSet::new();
        for node in parents {
            if let Some(parent) = node.parent() {
                tmp_parents.insert(parent);
            }

            // calculate accumulation hash for this node
            if hash_table.contains_key(&node.id()) {
                continue;
            }
            let mut acc = Digest([0; 16]);
            for child in node.children() {
                if let Some(hash) = hash_table.get(&child.id()) {
                    acc = acc.concat(*hash);
                } else {
                    let hash = child.hash();
                    hash_table.insert(child.id(), hash);
                    acc = acc.concat(hash);
                }
            }
            acc = acc.concat(node.hash());
            hash_table.insert(node.id(), acc);
        }
        parents = tmp_parents;
    }
    hash_table
}

#[cfg(test)]
mod test {
    use std::fs;
    use tree::XTreePrintOptions;

    use super::*;

    #[test]
    fn test_calculate_hash_table() {
        let text1 = fs::read_to_string("file1.xml").unwrap();
        let tree1 = XTree::parse(&text1).unwrap();
        let ht1 = calculate_hash_table(&tree1);
        let hex_marker1 = ht1
            .iter()
            .map(|(k, v)| (*k, format!("{} - {:x}", k, v)))
            .collect();
        let s1 = tree1.print_to_str(XTreePrintOptions::default().with_node_marker(&hex_marker1));
        println!("{s1}");

        let text2 = fs::read_to_string("file1.xml").unwrap();
        let tree2 = XTree::parse(&text2).unwrap();
        let ht2 = calculate_hash_table(&tree2);
        let hex_marker2 = ht2
            .iter()
            .map(|(k, v)| (*k, format!("{} - {:x}", k, v)))
            .collect();
        let s2 = tree2.print_to_str(XTreePrintOptions::default().with_node_marker(&hex_marker2));
        println!("{s2}");

        let text3 = fs::read_to_string("file1.xml").unwrap();
        let tree3 = XTree::parse(&text3).unwrap();
        let ht3 = calculate_hash_table(&tree3);
        let hex_marker3 = ht3
            .iter()
            .map(|(k, v)| (*k, format!("{} - {:x}", k, v)))
            .collect();
        let s3 = tree3.print_to_str(XTreePrintOptions::default().with_node_marker(&hex_marker3));
        println!("{s3}");

        assert_eq!(ht1.get(&tree1.root().id()), ht2.get(&tree2.root().id()));
    }
}
