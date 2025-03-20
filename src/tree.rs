use std::{collections::HashMap, fmt::Display};

use md5::Digest;
use roxmltree::{Document, Node, NodeId};

#[derive(Debug, thiserror::Error)]
pub enum XTreeError {
    #[error(transparent)]
    ParseError(#[from] roxmltree::Error),
}

pub struct XTree<'doc>(Document<'doc>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct XNode<'a, 'doc: 'a> {
    node: Node<'a, 'doc>,
    attr_name: Option<&'doc str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum XNodeId<'doc> {
    ElementOrText(NodeId),
    Attribute { node_id: NodeId, name: &'doc str },
}

impl<'doc> Display for XNodeId<'doc> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XNodeId::ElementOrText(node_id) => write!(f, "{}", node_id.get()),
            XNodeId::Attribute { node_id, name } => write!(f, "{}[{}]", node_id.get(), name),
        }
    }
}

impl<'doc> From<Document<'doc>> for XTree<'doc> {
    fn from(value: Document<'doc>) -> Self {
        Self(value)
    }
}

impl<'a, 'doc: 'a> XNode<'a, 'doc> {
    pub fn id(&'a self) -> XNodeId<'doc> {
        if let Some(name) = self.attr_name {
            XNodeId::Attribute {
                node_id: self.node.id(),
                name,
            }
        } else {
            XNodeId::ElementOrText(self.node.id())
        }
    }

    pub fn parent(&self) -> Option<Self> {
        if self.attr_name.is_some() {
            Some(Self {
                node: self.node.clone(),
                attr_name: None,
            })
        } else {
            self.node
                .parent()
                .filter(|p| !p.is_root())
                .map(|parent| Self {
                    node: parent,
                    attr_name: None,
                })
        }
    }

    pub fn children(&self) -> Vec<Self> {
        if self.attr_name.is_some() {
            return Vec::new();
        }
        let nodes = self
            .node
            .children()
            .filter(|node| !(node.is_text() && node.text().unwrap().trim().is_empty()))
            .map(|node| Self {
                node,
                attr_name: None,
            });
        let attrs = self.node.attributes().map(|attr| Self {
            node: self.node.clone(),
            attr_name: Some(attr.name()),
        });
        nodes.chain(attrs).collect()
    }

    pub fn value(&self) -> Option<&str> {
        if let Some(name) = self.attr_name {
            self.node.attribute(name)
        } else {
            self.node.text()
        }
    }

    pub fn range(&self) -> core::ops::Range<usize> {
        if let Some(name) = self.attr_name {
            self.node.attribute_node(name).unwrap().range()
        } else {
            self.node.range()
        }
    }

    pub(crate) fn hash(&self) -> Digest {
        if let Some(name) = self.attr_name {
            md5::compute(format!(
                "{}={}",
                name,
                self.node.attribute(name).unwrap_or_default()
            ))
        } else {
            match self.node.node_type() {
                roxmltree::NodeType::Element => {
                    let name = self.node.tag_name().name();
                    let namespace = self.node.tag_name().namespace().unwrap_or_default();
                    md5::compute(format!("{}:{}", namespace, name))
                }
                roxmltree::NodeType::Text => md5::compute(self.node.text().unwrap_or_default()),
                _ => unreachable!(),
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct XTreePrintOptions<'o> {
    node_marker: HashMap<XNodeId<'o>, String>,
    indent: usize,
}

impl<'o> Default for XTreePrintOptions<'o> {
    fn default() -> Self {
        Self {
            node_marker: HashMap::new(),
            indent: 3,
        }
    }
}

impl<'o> XTreePrintOptions<'o> {
    pub fn with_indent(mut self, n: usize) -> Self {
        assert!(n > 0);
        self.indent = n;
        self
    }

    pub fn with_node_marker<D: Display>(mut self, marker: &HashMap<XNodeId<'o>, D>) -> Self {
        let new_map = marker.iter().map(|(k, v)| (*k, v.to_string())).collect();
        self.node_marker = new_map;
        self
    }
}

impl<'a, 'doc: 'a> XTree<'doc> {
    pub fn parse(text: &'doc str) -> Result<Self, XTreeError> {
        Ok(Self::from(Document::parse(text)?))
    }

    pub fn print_to_str<'o>(&self, options: XTreePrintOptions<'o>) -> String {
        fn node_to_str(node: &XNode) -> String {
            if let Some(name) = node.attr_name {
                format!(
                    "{}: {}",
                    name,
                    node.node.attribute(name).unwrap_or_default()
                )
            } else {
                match node.node.node_type() {
                    roxmltree::NodeType::Element => format!("<{}>", node.node.tag_name().name()),
                    roxmltree::NodeType::Text => {
                        let text = node.node.text().unwrap_or_default();
                        format!(
                            "{:40?}{}",
                            text,
                            if text.chars().count() > 40 { "..." } else { "" }
                        )
                    }
                    _ => unreachable!(),
                }
            }
        }

        fn tree_to_str<'o>(
            pipes: &mut Vec<bool>,
            node: &XNode,
            options: &XTreePrintOptions<'o>,
        ) -> String {
            let marker = if let Some(m) = options.node_marker.get(&node.id()) {
                format!("[{}] ", m)
            } else {
                String::new()
            };
            let mut tree_str = if pipes.len() > 0 {
                let mut prefix = String::new();
                for pipe_char in &pipes[..pipes.len() - 1] {
                    prefix.push(if *pipe_char { '│' } else { ' ' });
                    prefix.push_str(&" ".repeat(options.indent - 1));
                }
                let suffix = if pipes[pipes.len() - 1] {
                    format!("├─{}{}", marker, node_to_str(node))
                } else {
                    format!("└─{}{}", marker, node_to_str(node))
                };
                format!("{}{}", prefix, suffix)
            } else {
                format!("{}{}", marker, node_to_str(node))
            };
            if node.children().len() == 0 {
                return tree_str;
            }
            let children = node.children();
            pipes.push(true);
            tree_str.push('\n');
            for child in &children[..children.len() - 1] {
                let line = tree_to_str(pipes, child, &options);
                tree_str.push_str(&line);
                tree_str.push('\n');
            }
            pipes.last_mut().map(|pipe| *pipe = false);
            let line = tree_to_str(pipes, children.last().unwrap(), &options);
            tree_str.push_str(&line);
            pipes.pop();
            tree_str
        }

        tree_to_str(&mut Vec::new(), &self.root(), &options)
    }

    pub fn get_node(&'doc self, id: XNodeId<'doc>) -> Option<XNode<'a, 'doc>> {
        match id {
            XNodeId::ElementOrText(node_id) => self.0.get_node(node_id).map(|node| XNode {
                node,
                attr_name: None,
            }),
            XNodeId::Attribute { node_id, name } => self.0.get_node(node_id).map(|node| XNode {
                node,
                attr_name: Some(name),
            }),
        }
    }

    pub fn root(&self) -> XNode {
        XNode {
            node: self.0.root_element(),
            attr_name: None,
        }
    }

    pub(crate) fn get_leaves_nodes(&'doc self) -> Vec<XNode<'a, 'doc>> {
        let attribute_nodes = self
            .0
            .descendants()
            .filter(|node| node.is_element())
            .flat_map(|node| {
                node.attributes().map(move |attr| XNode {
                    node: node.clone(),
                    attr_name: Some(attr.name()),
                })
            });
        let leaves = self
            .0
            .descendants()
            .filter(|node| (node.is_element() || node.is_text()) && !node.has_children())
            .filter(|node| !(node.is_text() && node.text().unwrap().trim().is_empty()))
            .map(|node| XNode {
                node,
                attr_name: None,
            });
        leaves.chain(attribute_nodes).collect()
    }
}

#[cfg(test)]
mod test {
    use std::fs;

    use super::*;

    #[test]
    fn test_print_tree() {
        let content = fs::read_to_string("file1.xml").unwrap();
        let tree = XTree::parse(&content).unwrap();
        let s = tree.print_to_str(Default::default());
        println!("{s}");
    }
}
