/*!
A library to compare XML files unorderedly.

This library implements the X-Diff algorithm from paper [X-Diff: An Effective Change Detection Algorithm for XML Documents](https://pages.cs.wisc.edu/~yuanwang/papers/xdiff.pdf).

## Example

```rust
use x_diff_rs::{
    diff,
    tree::{XTree, XTreePrintOptions},
};

fn main() {
    let text1 = r#"
<Profile>
 <Customer>
  <PersonName NameType="Default">
   <NameTitle>Mr.</NameTitle>
   <GivenName>George</GivenName>
   <MiddleName>A.</MiddleName>
   <SurName>Smith</SurName>
  </PersonName>
  <TelephoneInfo PhoneTech="Voice" PhoneUse="Work" >
   <Telephone> <AreaCityCode>206</AreaCityCode>
	<PhoneNumber>813-8698</PhoneNumber>
   </Telephone>
  </TelephoneInfo>
  <PaymentForm>
   ...
  </PaymentForm>
  <Address>
   <StreetNmbr POBox="4321-01">From hell</StreetNmbr>
   <BldgRoom>Suite 800</BldgRoom>
   <CityName>Seattle</CityName>
   <StateProv PostalCode="98108">WA</StateProv>
   <CountryName>USA</CountryName>
  </Address>
  <Address>
   <StreetNmbr POBox="4321-01">1200 Yakima St</StreetNmbr>
   <BldgRoom>Suite 800</BldgRoom>
   <CityName>Seattle</CityName>
   <StateProv PostalCode="98108">WA</StateProv>
   <CountryName>USA</CountryName>
  </Address>
 </Customer>
</Profile>
    "#;

    let text2 = r#"
<Profile>
 <Customer>
  <PersonName NameType="Default">
   <NameTitle>Mr.</NameTitle>
   <GivenName>George</GivenName>
   <MiddleName>A.</MiddleName>
   <SurName>Smith</SurName>
  </PersonName>
  <TelephoneInfo PhoneTech="Voice" PhoneUse="Work" >
   <Telephone> <AreaCityCode>206</AreaCityCode>
	<PhoneNumber>813-8698</PhoneNumber>
   </Telephone>
  </TelephoneInfo>
  <Address>
   <StreetNmbr POBox="4321-01">From hell</StreetNmbr>
   <BldgRoom>Suite 800</BldgRoom>
   <CityName>Seattle</CityName>
   <StateProv PostalCode="98108">WA</StateProv>
   <CountryName>USA</CountryName>
  </Address>
  <Address>
   <StreetNmbr POBox="1234-01">1200 Yakima St</StreetNmbr>
   <BldgRoom>Suite 800</BldgRoom>
   <CityName>Paris</CityName>
   <StateProv PostalCode="98108">WA</StateProv>
   <CountryName>USA</CountryName>
  </Address>
  <Status>Single</Status>
 </Customer>
</Profile>
    "#;
    let tree1 = XTree::parse(&text1).unwrap();
    let tree2 = XTree::parse(&text2).unwrap();
    tree1.print(XTreePrintOptions::default().with_node_id());
    tree2.print(XTreePrintOptions::default().with_node_id());
    let difference = diff(&tree1, &tree2);
    for d in difference {
        println!("{d}");
    }
}
```
*/

use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use md5::Digest;
use tree::{XNode, XNodeId, XTree};

/// XML parsing and tree operations.
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

#[derive(Debug, Clone)]
pub enum Edit<'tree1, 'tree2> {
    Insert {
        child_node: XNodeId<'tree2>,
        to: XNodeId<'tree1>,
    },
    Delete(XNodeId<'tree1>),
    Update {
        node_id: XNodeId<'tree1>,
        old_value: String,
        new_value: String,
    },
    ReplaceRoot,
}

impl Display for Edit<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Edit::Insert { child_node, to } => {
                write!(f, "insert node {} to node {}", child_node, to)
            }
            Edit::Delete(node_id) => write!(f, "delete node {}", node_id),
            Edit::Update {
                node_id,
                old_value,
                new_value,
            } => write!(
                f,
                "update node {}: {:?} -> {:?}",
                node_id, old_value, new_value
            ),
            Edit::ReplaceRoot => write!(f, "replace root node"),
        }
    }
}

/// Calculate the difference between two XML trees, represented by the minum edit operations to transform `tree1` to `tree2`.
pub fn diff<'doc1, 'doc2>(
    tree1: &'doc1 XTree<'doc1>,
    tree2: &'doc2 XTree<'doc2>,
) -> Vec<Edit<'doc1, 'doc2>> {
    fn diff_node<'doc1, 'doc2>(
        node1: XNode<'_, 'doc1>,
        ht1: &HashMap<XNodeId<'doc1>, Digest>,
        node2: XNode<'_, 'doc2>,
        ht2: &HashMap<XNodeId<'doc2>, Digest>,
    ) -> Vec<Edit<'doc1, 'doc2>> {
        if ht1.get(&node1.id()) == ht2.get(&node2.id()) {
            return Vec::new();
        }

        // Leaf nodes with different hashes mean different values
        if (node1.is_attribute() && node2.is_attribute()) || (node1.is_text() && node2.is_text()) {
            return vec![Edit::Update {
                node_id: node1.id(),
                old_value: node1.value().unwrap_or_default().trim().to_string(),
                new_value: node2.value().unwrap_or_default().trim().to_string(),
            }];
        }

        let mut iht1: HashMap<_, _> = node1
            .children()
            .iter()
            .map(|n| (*ht1.get(&n.id()).unwrap(), *n))
            .collect();
        let mut iht2: HashMap<_, _> = node2
            .children()
            .iter()
            .map(|n| (*ht2.get(&n.id()).unwrap(), *n))
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
            diff.push(Edit::Delete(n1.id()));
        }
        for n2 in remaining_children2 {
            diff.push(Edit::Insert {
                child_node: n2.id(),
                to: node1.id(),
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

fn calculate_hash_table<'doc>(tree: &'doc XTree) -> HashMap<XNodeId<'doc>, Digest> {
    fn hash_of_node<'doc>(
        node: XNode<'_, 'doc>,
        ht: &mut HashMap<XNodeId<'doc>, Digest>,
    ) -> Digest {
        let hash = if node.children().is_empty() {
            node.hash()
        } else {
            let mut acc = node.hash();
            for child in node.children() {
                acc = acc.concat(hash_of_node(child, ht));
            }
            acc
        };
        ht.insert(node.id(), hash);
        hash
    }
    let mut hash_table = HashMap::new();
    hash_of_node(tree.root(), &mut hash_table);
    hash_table
}

#[cfg(test)]
mod test {
    use std::fs;
    use tree::XTreePrintOptions;

    use super::*;

    #[test]
    fn test_calculate_hash_table_same_tree() {
        let text1 = fs::read_to_string("test/file1.xml").unwrap();
        let tree1 = XTree::parse(&text1).unwrap();
        let ht1 = calculate_hash_table(&tree1);

        let text2 = fs::read_to_string("test/file1.xml").unwrap();
        let tree2 = XTree::parse(&text2).unwrap();
        let ht2 = calculate_hash_table(&tree2);

        assert_eq!(ht1.get(&tree1.root().id()), ht2.get(&tree2.root().id()));
    }

    #[test]
    fn test_calculate_hash_table_different_tree() {
        let text1 = fs::read_to_string("test/file1.xml").unwrap();
        let tree1 = XTree::parse(&text1).unwrap();
        let ht1 = calculate_hash_table(&tree1);
        let hex_marker1 = ht1.iter().map(|(k, v)| (*k, format!("{:x}", v))).collect();
        tree1.print(
            XTreePrintOptions::default()
                .with_node_marker(&hex_marker1)
                .with_node_id(),
        );

        let text2 = fs::read_to_string("test/file2.xml").unwrap();
        let tree2 = XTree::parse(&text2).unwrap();
        let ht2 = calculate_hash_table(&tree2);
        let hex_marker2 = ht2.iter().map(|(k, v)| (*k, format!("{:x}", v))).collect();
        tree2.print(
            XTreePrintOptions::default()
                .with_node_marker(&hex_marker2)
                .with_node_id(),
        );

        assert_ne!(ht1.get(&tree1.root().id()), ht2.get(&tree2.root().id()));
    }

    #[test]
    fn test_diff() {
        let text1 = fs::read_to_string("test/file1.xml").unwrap();
        let tree1 = XTree::parse(&text1).unwrap();
        tree1.print(XTreePrintOptions::default().with_node_id());

        let text2 = fs::read_to_string("test/file2.xml").unwrap();
        let tree2 = XTree::parse(&text2).unwrap();
        tree2.print(XTreePrintOptions::default().with_node_id());

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
