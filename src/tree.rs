use md5::Digest;
use roxmltree::{Document, ExpandedName, Node, NodeId};
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
        } else if self.is_text() {
            XNodeName::Text
        } else {
            XNodeName::TagName(self.node.tag_name())
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

impl<'a, 'doc: 'a> XTree<'doc> {
    /// Parse XML to tree structure.
    pub fn parse(text: &'doc str) -> Result<Self, XTreeError> {
        Ok(Self::from(
            Document::parse(text).map_err(XTreeError::ParseError)?,
        ))
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

#[cfg(feature = "print")]
pub mod print {
    use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

    use crate::diff::{Edit, diff};

    use super::{XNode, XNodeId, XTree};
    use std::{collections::HashMap, fmt::Display};

    #[derive(Debug, Clone)]
    pub struct PrintTreeOptions<'a> {
        marker: HashMap<XNodeId<'a>, String>,
        with_id: bool,
        indent: usize,
    }

    #[derive(Debug, Clone)]
    pub struct PrintTreeDiffOptions {
        indent: usize,
        color: bool,
    }

    #[derive(Debug, Clone, Copy)]
    enum GutterKind {
        None,
        Blank,
        Add,
        Delete,
    }

    impl GutterKind {
        fn symbol(&self) -> &'static str {
            match self {
                GutterKind::None => "",
                GutterKind::Blank => " ",
                GutterKind::Add => "+",
                GutterKind::Delete => "-",
            }
        }
    }

    impl Default for PrintTreeDiffOptions {
        fn default() -> Self {
            Self {
                indent: 3,
                color: true,
            }
        }
    }

    impl PrintTreeDiffOptions {
        pub fn indent(mut self, n: usize) -> Self {
            self.indent = n;
            self
        }

        pub fn with_color(mut self, yes: bool) -> Self {
            self.color = yes;
            self
        }
    }

    pub fn write_tree_diff<W: WriteColor>(
        w: &mut W,
        tree1: &XTree,
        tree2: &XTree,
        options: PrintTreeDiffOptions,
    ) -> std::io::Result<()> {
        let edits = diff(tree1, tree2);

        // trees are the same
        if edits.is_empty() {
            return write!(w, "The trees are the same.");
        }

        // trees are completely different
        if matches!(edits[0], Edit::ReplaceRoot) {
            let mut vlines = Vec::new();
            write_subtree(
                w,
                tree1.root(),
                &PrintTreeOptions::default().with_indent(options.indent),
                GutterKind::Delete,
                &mut vlines,
            )?;
            return write_subtree(
                w,
                tree2.root(),
                &PrintTreeOptions::default().with_indent(options.indent),
                GutterKind::Add,
                &mut vlines,
            );
        }

        let mut changed_nodes = HashMap::new();
        for e in edits {
            let key = match e {
                crate::diff::Edit::Insert {
                    child_node: _,
                    to_node,
                } => to_node.id(),
                crate::diff::Edit::Delete(node) => node.id(),
                crate::diff::Edit::Update { old, new: _ } => old.id(),
                crate::diff::Edit::ReplaceRoot => unreachable!(),
            };
            changed_nodes.entry(key).or_insert(Vec::new()).push(e);
        }

        let mut vlines = Vec::new();
        write_subtree_diff(w, tree1.root(), &changed_nodes, &options, &mut vlines)
    }

    fn write_subtree_diff<W: WriteColor>(
        w: &mut W,
        node: XNode,
        changed_nodes: &HashMap<XNodeId, Vec<Edit>>,
        options: &PrintTreeDiffOptions,
        vlines: &mut Vec<bool>,
    ) -> std::io::Result<()> {
        if let Some(edits) = changed_nodes.get(&node.id()) {
            if matches!(edits[0], Edit::Insert { .. }) {
                write_node_line(
                    w,
                    node,
                    &PrintTreeOptions::default().with_indent(options.indent),
                    GutterKind::Blank,
                    vlines,
                )?;
                let children = node.children();
                if children.is_empty() {
                    return Ok(());
                }
                vlines.push(true);
                for child in children {
                    write_subtree_diff(w, child, &changed_nodes, options, vlines)?;
                }
            }
            let last_index = edits.len() - 1;
            for (i, e) in edits.into_iter().enumerate() {
                match e {
                    Edit::Insert {
                        child_node,
                        to_node: _,
                    } => {
                        if i == last_index {
                            *vlines.last_mut().unwrap() = false;
                        }
                        write_subtree(
                            w,
                            *child_node,
                            &PrintTreeOptions::default().with_indent(options.indent),
                            GutterKind::Add,
                            vlines,
                        )?;
                    }
                    Edit::Delete(_) => write_subtree(
                        w,
                        node,
                        &PrintTreeOptions::default().with_indent(options.indent),
                        GutterKind::Delete,
                        vlines,
                    )?,
                    Edit::Update { old, new } => {
                        write_subtree(
                            w,
                            *old,
                            &PrintTreeOptions::default().with_indent(options.indent),
                            GutterKind::Delete,
                            vlines,
                        )?;
                        write_subtree(
                            w,
                            *new,
                            &PrintTreeOptions::default().with_indent(options.indent),
                            GutterKind::Add,
                            vlines,
                        )?;
                    }
                    Edit::ReplaceRoot => unreachable!(),
                }
            }
            if matches!(edits[0], Edit::Insert { .. }) {
                vlines.pop();
            }
        } else {
            write_node_line(
                w,
                node,
                &PrintTreeOptions::default().with_indent(options.indent),
                GutterKind::Blank,
                vlines,
            )?;
            let children = node.children();
            if children.is_empty() {
                return Ok(());
            }
            vlines.push(true);
            let last_index = children.len() - 1;
            for (i, child) in children.into_iter().enumerate() {
                if i == last_index {
                    *vlines.last_mut().unwrap() = false;
                }
                write_subtree_diff(w, child, &changed_nodes, options, vlines)?;
            }
            vlines.pop();
        }
        Ok(())
    }

    impl Default for PrintTreeOptions<'_> {
        fn default() -> Self {
            Self {
                marker: HashMap::new(),
                indent: 3,
                with_id: false,
            }
        }
    }

    impl<'a> PrintTreeOptions<'a> {
        pub fn with_indent(mut self, n: usize) -> Self {
            assert!(n > 0);
            self.indent = n;
            self
        }

        /// Attach markers to nodes while printing. The marker will be wrapped around `()`.
        pub fn with_marker<D: Display>(mut self, marker: &HashMap<XNodeId<'a>, D>) -> Self {
            let new_map = marker.iter().map(|(k, v)| (*k, v.to_string())).collect();
            self.marker = new_map;
            self
        }

        /// Attach ID to nodes while printing. The node id will be wrapped around `[]`.
        pub fn with_node_id(mut self) -> Self {
            self.with_id = true;
            self
        }
    }

    pub fn print_tree(tree: &XTree, options: PrintTreeOptions) {
        let mut stdout = StandardStream::stdout(ColorChoice::Never);
        write_tree(&mut stdout, tree, options).unwrap();
    }

    pub fn print_tree_diff(tree1: &XTree, tree2: &XTree, options: PrintTreeDiffOptions) {
        let mut stdout = StandardStream::stdout(if options.color {
            ColorChoice::Always
        } else {
            ColorChoice::Never
        });
        write_tree_diff(&mut stdout, tree1, tree2, options).unwrap();
    }

    pub fn write_tree<W: WriteColor>(
        w: &mut W,
        tree: &XTree,
        options: PrintTreeOptions,
    ) -> std::io::Result<()> {
        let mut vlines = Vec::new();
        write_subtree(w, tree.root(), &options, GutterKind::None, &mut vlines)
    }

    fn node_text_prefix(node: &XNode, with_id: bool, marker: &HashMap<XNodeId, String>) -> String {
        let id_str = if with_id {
            format!("[{}] ", node.id())
        } else {
            String::new()
        };
        let m = if let Some(m) = marker.get(&node.id()) {
            format!("({}) ", m)
        } else {
            String::new()
        };
        format!("{}{}", id_str, m)
    }

    fn node_text(node: &XNode, prefix: &str) -> String {
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
                    let text = node.node.text().unwrap_or_default().trim();
                    let mut short_text: String = text.chars().take(40).collect();
                    if text.chars().count() > 40 {
                        short_text.push_str("...");
                    }
                    format!("{:?}", short_text)
                }
                _ => unreachable!(),
            }
        };
        format!("{}{}", prefix, node_str)
    }

    fn set_color<W: WriteColor>(w: &mut W, gutter: GutterKind) -> std::io::Result<()> {
        match gutter {
            GutterKind::None => w.reset(),
            GutterKind::Blank => w.reset(),
            GutterKind::Add => w.set_color(ColorSpec::new().set_fg(Some(Color::Green))),
            GutterKind::Delete => w.set_color(ColorSpec::new().set_fg(Some(Color::Red))),
        }
    }

    fn write_node_line<W: WriteColor>(
        w: &mut W,
        node: XNode,
        options: &PrintTreeOptions<'_>,
        gutter: GutterKind,
        vlines: &mut Vec<bool>,
    ) -> std::io::Result<()> {
        set_color(w, gutter)?;
        let gutter_str = gutter.symbol();
        let node_prefix = node_text_prefix(&node, options.with_id, &options.marker);
        let node_line = if !vlines.is_empty() {
            let mut prefix = String::new();
            for pipe_char in &vlines[..vlines.len() - 1] {
                prefix.push(if *pipe_char { '│' } else { ' ' });
                prefix.push_str(&" ".repeat(options.indent - 1));
            }
            let suffix = if vlines[vlines.len() - 1] {
                format!("├─{}", node_text(&node, &node_prefix))
            } else {
                format!("└─{}", node_text(&node, &node_prefix))
            };
            format!("{}{}", prefix, suffix)
        } else {
            format!("{}", node_text(&node, &node_prefix))
        };
        writeln!(w, "{}{}", gutter_str, node_line)?;
        w.reset()
    }

    fn write_subtree<W: WriteColor>(
        w: &mut W,
        node: XNode,
        options: &PrintTreeOptions<'_>,
        gutter: GutterKind,
        vlines: &mut Vec<bool>,
    ) -> std::io::Result<()> {
        set_color(w, gutter)?;
        write_node_line(w, node, options, gutter, vlines)?;
        let children = node.children();
        if children.is_empty() {
            return Ok(());
        }
        vlines.push(true);
        let last_index = children.len() - 1;
        for (i, child) in children.into_iter().enumerate() {
            if i == last_index {
                *vlines.last_mut().unwrap() = false;
            }
            write_subtree(w, child, options, gutter, vlines)?;
        }
        vlines.pop();
        w.reset()?;
        Ok(())
    }

    #[cfg(test)]
    mod test {
        use std::{fs, io::Cursor};

        use termcolor::NoColor;

        use super::*;
        #[test]
        fn test_print_tree() {
            let content = fs::read_to_string("test/file1.xml").unwrap();
            let tree = XTree::parse(&content).unwrap();
            let mut buffer = Vec::new();
            let cursor = Cursor::new(&mut buffer);
            let mut no_color = NoColor::new(cursor);
            write_tree(&mut no_color, &tree, PrintTreeOptions::default()).unwrap();
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
   │  ├─<Bio>
   │  │  └─"A skilled engineer with a passion for so..."
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
   │  └─"..."
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
            assert_eq!(expected.trim(), String::from_utf8_lossy(&buffer).trim());
        }

        #[test]
        fn test_print_diff() {
            let text1 = fs::read_to_string("test/file1.xml").unwrap();
            let tree1 = XTree::parse(&text1).unwrap();
            let text2 = fs::read_to_string("test/file2.xml").unwrap();
            let tree2 = XTree::parse(&text2).unwrap();
            print_tree_diff(&tree1, &tree2, PrintTreeDiffOptions::default());
        }
    }
}
