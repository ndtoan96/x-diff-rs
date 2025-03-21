use md5::Digest;
use roxmltree::{Document, ExpandedName, Node, NodeId};
#[cfg(feature = "print")]
use std::collections::HashMap;
use std::{borrow::Cow, fmt::Display};

#[derive(Debug, Clone)]
pub enum XTreeError {
    ParseError(roxmltree::Error),
}

/// A tree representation of the XML format. It is a wrapper around [roxmltree::Document]
#[derive(Debug)]
pub struct XTree<'doc>(Document<'doc>);

/// A node in the XML tree. It can be an element node, an attribute node, or a text node.
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

#[derive(Debug, Clone)]
pub enum XNodeName<'a, 'b> {
    TagName(ExpandedName<'a, 'b>),
    AttributeName(&'a str),
    Text,
}

impl Display for XNodeId<'_> {
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
    /// Get node id.
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

    /// Get node name.
    pub fn name(&self) -> XNodeName {
        if let Some(name) = self.attr_name {
            XNodeName::AttributeName(name)
        } else {
            if self.is_text() {
                XNodeName::Text
            } else {
                XNodeName::TagName(self.node.tag_name())
            }
        }
    }

    /// Get the parent node.
    pub fn parent(&self) -> Option<Self> {
        if self.attr_name.is_some() {
            Some(Self {
                node: self.node,
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

    /// Get the children nodes.
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
            node: self.node,
            attr_name: Some(attr.name()),
        });
        nodes.chain(attrs).collect()
    }

    pub fn is_attribute(&self) -> bool {
        self.attr_name.is_some()
    }

    pub fn is_text(&self) -> bool {
        self.attr_name.is_none() && self.node.is_text()
    }

    pub fn is_element(&self) -> bool {
        self.attr_name.is_none() && self.node.is_element()
    }

    /// Get the node value. Only attribute and text node have value.
    pub fn value(&self) -> Option<&str> {
        if let Some(name) = self.attr_name {
            self.node.attribute(name)
        } else {
            self.node.text()
        }
    }

    /// Get the byte range of this node from the original text.
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
                self.node.attribute(name).unwrap_or_default().trim()
            ))
        } else {
            match self.node.node_type() {
                roxmltree::NodeType::Element => {
                    let name = self.node.tag_name().name();
                    let namespace = self.node.tag_name().namespace().unwrap_or_default();
                    md5::compute(format!("{}:{}", namespace, name))
                }
                roxmltree::NodeType::Text => {
                    md5::compute(self.node.text().unwrap_or_default().trim())
                }
                _ => unreachable!(),
            }
        }
    }

    pub(crate) fn signature(&self) -> Cow<str> {
        if let Some(name) = self.attr_name {
            Cow::Borrowed(name)
        } else {
            match self.node.node_type() {
                roxmltree::NodeType::Element => Cow::Owned(format!(
                    "{}:{}",
                    self.node.tag_name().namespace().unwrap_or_default(),
                    self.node.tag_name().name()
                )),
                roxmltree::NodeType::Text => Cow::Borrowed("text"),
                _ => unreachable!(),
            }
        }
    }
}

/// Options for priting the XML tree
#[cfg(feature = "print")]
#[derive(Debug, Clone)]
pub struct XTreePrintOptions<'o> {
    node_marker: HashMap<XNodeId<'o>, String>,
    with_id: bool,
    indent: usize,
}

#[cfg(feature = "print")]
impl Default for XTreePrintOptions<'_> {
    fn default() -> Self {
        Self {
            node_marker: HashMap::new(),
            indent: 3,
            with_id: false,
        }
    }
}

#[cfg(feature = "print")]
impl<'o> XTreePrintOptions<'o> {
    pub fn with_indent(mut self, n: usize) -> Self {
        assert!(n > 0);
        self.indent = n;
        self
    }

    /// Attach markers to nodes while printing. The marker will be wrapped around `()`.
    pub fn with_node_marker<D: Display>(mut self, marker: &HashMap<XNodeId<'o>, D>) -> Self {
        let new_map = marker.iter().map(|(k, v)| (*k, v.to_string())).collect();
        self.node_marker = new_map;
        self
    }

    /// Attach ID to nodes while printing. The node id will be wrapped around `[]`.
    pub fn with_node_id(mut self) -> Self {
        self.with_id = true;
        self
    }
}

impl<'a, 'doc: 'a> XTree<'doc> {
    /// Parse XML to tree structure.
    pub fn parse(text: &'doc str) -> Result<Self, XTreeError> {
        Ok(Self::from(
            Document::parse(text).map_err(|e| XTreeError::ParseError(e))?,
        ))
    }

    /// Print the tree to stdout.
    #[cfg(feature = "print")]
    pub fn print(&self, options: XTreePrintOptions<'_>) {
        println!("{}", self.print_to_str(options));
    }

    /// Print the tree to a String.
    #[cfg(feature = "print")]
    pub fn print_to_str(&self, options: XTreePrintOptions<'_>) -> String {
        fn node_to_str(node: &XNode, options: &XTreePrintOptions) -> String {
            let id_str = if options.with_id {
                format!("[{}] ", node.id())
            } else {
                String::new()
            };
            let marker = if let Some(m) = options.node_marker.get(&node.id()) {
                format!("({}) ", m)
            } else {
                String::new()
            };
            let node_str = if let Some(name) = node.attr_name {
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
            };
            format!("{}{}{}", marker, id_str, node_str)
        }

        fn tree_to_str(
            pipes: &mut Vec<bool>,
            node: &XNode,
            options: &XTreePrintOptions<'_>,
        ) -> String {
            let mut tree_str = if !pipes.is_empty() {
                let mut prefix = String::new();
                for pipe_char in &pipes[..pipes.len() - 1] {
                    prefix.push(if *pipe_char { '│' } else { ' ' });
                    prefix.push_str(&" ".repeat(options.indent - 1));
                }
                let suffix = if pipes[pipes.len() - 1] {
                    format!("├─{}", node_to_str(node, options))
                } else {
                    format!("└─{}", node_to_str(node, options))
                };
                format!("{}{}", prefix, suffix)
            } else {
                format!("{}", node_to_str(node, options))
            };
            if node.children().is_empty() {
                return tree_str;
            }
            let children = node.children();
            pipes.push(true);
            tree_str.push('\n');
            for child in &children[..children.len() - 1] {
                let line = tree_to_str(pipes, child, options);
                tree_str.push_str(&line);
                tree_str.push('\n');
            }
            *pipes.last_mut().unwrap() = false;
            let line = tree_to_str(pipes, children.last().unwrap(), options);
            tree_str.push_str(&line);
            pipes.pop();
            tree_str
        }

        tree_to_str(&mut Vec::new(), &self.root(), &options)
    }

    /// Get an [XNode] from [XNodeId].
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

    /// Get the root node.
    pub fn root(&self) -> XNode {
        XNode {
            node: self.0.root_element(),
            attr_name: None,
        }
    }

    /// Get the underlying roxmltree::Document.
    pub fn get_roxmltree_doc(self) -> roxmltree::Document<'doc> {
        self.0
    }
}

#[cfg(all(test, feature = "print"))]
mod test {
    use std::fs;

    use super::*;

    #[test]
    fn test_print_tree() {
        let content = fs::read_to_string("test/file1.xml").unwrap();
        let tree = XTree::parse(&content).unwrap();
        let s = tree.print_to_str(XTreePrintOptions::default());
        let expected = r#"
<Profile>
└─<Customer>
   ├─<PersonName>
   │  ├─<NameTitle>
   │  │  └─"Mr."
   │  ├─<GivenName>
   │  │  └─"George"
   │  ├─<MiddleName>
   │  │  └─"A."
   │  ├─<SurName>
   │  │  └─"Smith"
   │  └─NameType: Default
   ├─<TelephoneInfo>
   │  ├─<Telephone>
   │  │  ├─<AreaCityCode>
   │  │  │  └─"206"
   │  │  └─<PhoneNumber>
   │  │     └─"813-8698"
   │  ├─PhoneTech: Voice
   │  └─PhoneUse: Work
   ├─<PaymentForm>
   │  └─"\n   ...\n  "
   ├─<Address>
   │  ├─<StreetNmbr>
   │  │  ├─"From hell"
   │  │  └─POBox: 4321-01
   │  ├─<BldgRoom>
   │  │  └─"Suite 800"
   │  ├─<CityName>
   │  │  └─"Seattle"
   │  ├─<StateProv>
   │  │  ├─"WA"
   │  │  └─PostalCode: 98108
   │  └─<CountryName>
   │     └─"USA"
   └─<Address>
      ├─<StreetNmbr>
      │  ├─"1200 Yakima St"
      │  └─POBox: 4321-01
      ├─<BldgRoom>
      │  └─"Suite 800"
      ├─<CityName>
      │  └─"Seattle"
      ├─<StateProv>
      │  ├─"WA"
      │  └─PostalCode: 98108
      └─<CountryName>
         └─"USA"
        "#;
        println!("{s}");

        assert_eq!(s.trim(), expected.trim());
    }
}
