/// A trait for converting a tree-sitter node into a Rust type.
pub trait FromNode {
    fn from_node(node: &tree_sitter::Node, source: &str) -> Option<Self>
    where
        Self: Sized;
}
