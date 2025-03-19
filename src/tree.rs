use md5::Digest;
use roxmltree::{Document, Node, NodeId};

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

impl<'a, 'doc: 'a> XTree<'doc> {
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
