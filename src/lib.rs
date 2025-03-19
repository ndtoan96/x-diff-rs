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

    use roxmltree::Document;

    use super::*;

    #[test]
    fn test_calculate_hash_table() {
        let text1 = fs::read_to_string("file1.xml").unwrap();
        let text2 = fs::read_to_string("file2.xml").unwrap();
        let doc1 = Document::parse(&text1).unwrap();
        let doc2 = Document::parse(&text2).unwrap();
        let tree1 = XTree::from(doc1);
        let tree2 = XTree::from(doc2);
        let ht1 = calculate_hash_table(&tree1);
        let ht2 = calculate_hash_table(&tree2);
        let set: HashSet<_> = ht1.values().collect();
        for (k, v) in ht2 {
            if set.contains(&v) {
                let node = tree2.get_node(k).unwrap();
                let range = node.range();
                let bytes = text2.as_bytes();
                let sub_str = String::from_utf8_lossy(&bytes[range]);
                println!("------");
                println!("{}", sub_str);
                if sub_str.trim() == "" {
                    dbg!(node);
                }
            }
        }
    }
}
