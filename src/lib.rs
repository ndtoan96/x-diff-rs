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
