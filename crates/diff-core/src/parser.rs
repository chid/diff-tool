//! Tree-sitter AST parsing and node flattening.
//!
//! Language detection, CST parsing, and flat AstNode record production
//! for the syntax-aware diff engine.

use std::path::Path;

/// Languages supported by the syntax-aware diff engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    TypeScript,
    Python,
}

impl Language {
    /// Resolve a language from a user-supplied string (case-insensitive).
    pub fn from_name(s: &str) -> Option<Language> {
        match s.to_lowercase().as_str() {
            "rust" | "rs" => Some(Language::Rust),
            "typescript" | "ts" | "tsx" => Some(Language::TypeScript),
            "python" | "py" => Some(Language::Python),
            _ => None,
        }
    }
}

/// AST node granularity for the syntax diff engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyntaxDepth {
    /// Function/class/module/trait-level nodes only (coarsest).
    Function,
    /// Declaration, definition, and control-flow nodes (default).
    Statement,
    /// All named nodes including identifiers and expressions (finest).
    Token,
}

impl SyntaxDepth {
    pub fn from_name(s: &str) -> Option<SyntaxDepth> {
        match s.to_lowercase().as_str() {
            "function" => Some(SyntaxDepth::Function),
            "statement" => Some(SyntaxDepth::Statement),
            "token" | "all" => Some(SyntaxDepth::Token),
            _ => None,
        }
    }
}

impl Default for SyntaxDepth {
    fn default() -> Self {
        SyntaxDepth::Statement
    }
}

/// A flattened AST node record from a tree-sitter parse.
#[derive(Debug, Clone)]
pub struct AstNode {
    pub id: usize,
    pub kind: String,
    pub kind_id: u16,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub text: String,
    pub hash: u64,
    pub named: bool,
}

impl Language {
    pub fn grammar(&self) -> tree_sitter::Language {
        match self {
            Language::Rust => tree_sitter_rust::LANGUAGE.into(),
            Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Language::Python => tree_sitter_python::LANGUAGE.into(),
        }
    }
}

/// Detect supported language from a file path's extension.
pub fn language_from_extension(path: &Path) -> Option<Language> {
    match path.extension()?.to_str()? {
        "rs" => Some(Language::Rust),
        "ts" | "tsx" => Some(Language::TypeScript),
        "py" => Some(Language::Python),
        _ => None,
    }
}

/// Parse source text into a flat Vec of AstNode records at the given depth.
pub fn parse_file(source: &str, lang: Language, depth: SyntaxDepth) -> Result<Vec<AstNode>, String> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&lang.grammar())
        .map_err(|e| format!("failed to set language: {}", e))?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| "failed to parse source".to_string())?;

    let mut nodes: Vec<AstNode> = Vec::new();
    walk_node(&tree.root_node(), source, None, depth, &mut nodes);
    Ok(nodes)
}

/// Recursively walk the tree-sitter CST, collecting named nodes at the
/// requested depth into a flat Vec with parent-child relationships.
fn walk_node(
    node: &tree_sitter::Node,
    source: &str,
    parent_id: Option<usize>,
    depth: SyntaxDepth,
    nodes: &mut Vec<AstNode>,
) {
    if !node.is_named() {
        // Skip anonymous nodes but recurse into children.
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                walk_node(&cursor.node(), source, parent_id, depth, nodes);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        return;
    }

    let keep = is_node_at_depth(node, depth);
    let node_id = if keep {
        let id = nodes.len();
        let text = &source[node.start_byte()..node.end_byte()];
        let ast_node = AstNode {
            id,
            kind: node.kind().to_string(),
            kind_id: node.kind_id(),
            parent: parent_id,
            children: Vec::new(),
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
            text: text.to_string(),
            hash: whitespace_normalized_hash(text),
            named: true,
        };
        nodes.push(ast_node);
        Some(id)
    } else {
        None
    };

    // Recurse into children.  Kept nodes pass their id as parent;
    // skipped nodes pass the grandparent id so depth remains correct.
    let next_parent = node_id.or(parent_id);
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            walk_node(&cursor.node(), source, next_parent, depth, nodes);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

/// Returns true if this named node should be kept at the given depth.
///
/// Depth levels:
/// - `Function`: only function/class/trait/module-level structures
/// - `Statement` (default): declarations, definitions, control flow
/// - `Token`: all named nodes
fn is_node_at_depth(node: &tree_sitter::Node, depth: SyntaxDepth) -> bool {
    debug_assert!(node.is_named());
    let kind = node.kind();
    match depth {
        SyntaxDepth::Token => true,
        SyntaxDepth::Statement => {
            // Exclude fine-grained identifiers, literals, error nodes,
            // and the root source_file wrapper.
            !matches!(
                kind,
                "identifier"
                    | "variable_name"
                    | "type_identifier"
                    | "escape_sequence"
                    | "string_content"
                    | "comment"
                    | "line_comment"
                    | "block_comment"
                    | "ERROR"
                    | "MISSING"
                    | "source_file"
            )
        }
        SyntaxDepth::Function => {
            matches!(
                kind,
                "function_item"
                    | "function_declaration"
                    | "function_definition"
                    | "method_definition"
                    | "class_definition"
                    | "class_declaration"
                    | "struct_item"
                    | "trait_item"
                    | "impl_item"
                    | "module_item"
                    | "interface_declaration"
                    | "enum_declaration"
            )
        }
    }
}

/// Compute a 64-bit hash of whitespace-normalized text.
pub fn whitespace_normalized_hash(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let normalized: String = text
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    normalized.hash(&mut hasher);
    hasher.finish()
}

/// Register child relationships after all nodes are collected.
///
/// Walks the flat list and assigns children based on parent pointers.
pub fn build_child_links(nodes: &mut Vec<AstNode>) {
    // Collect child IDs for each parent.
    let child_map: Vec<(Option<usize>, usize)> =
        nodes.iter().map(|n| (n.parent, n.id)).collect();
    for &(parent_id, child_id) in &child_map {
        if let Some(pid) = parent_id {
            if pid < nodes.len() {
                nodes[pid].children.push(child_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_language_from_extension() {
        assert_eq!(
            language_from_extension(Path::new("foo.rs")),
            Some(Language::Rust)
        );
        assert_eq!(
            language_from_extension(Path::new("bar.ts")),
            Some(Language::TypeScript)
        );
        assert_eq!(
            language_from_extension(Path::new("baz.tsx")),
            Some(Language::TypeScript)
        );
        assert_eq!(
            language_from_extension(Path::new("qux.py")),
            Some(Language::Python)
        );
        assert_eq!(language_from_extension(Path::new("readme.md")), None);
        assert_eq!(language_from_extension(Path::new("Makefile")), None);
    }

    #[test]
    fn parses_rust_function() {
        let source = "fn hello() -> i32 { 42 }";
        let mut nodes = parse_file(source, Language::Rust, SyntaxDepth::Statement).unwrap();
        build_child_links(&mut nodes);
        assert!(
            nodes.iter().any(|n| n.kind == "function_item"),
            "expected function_item node, got kinds: {:?}",
            nodes.iter().map(|n| &n.kind).collect::<Vec<_>>()
        );
    }

    #[test]
    fn parses_python_function() {
        let source = "def hello():\n    pass";
        let mut nodes = parse_file(source, Language::Python, SyntaxDepth::Statement).unwrap();
        build_child_links(&mut nodes);
        assert!(
            nodes.iter().any(|n| n.kind == "function_definition"),
            "expected function_definition node, got kinds: {:?}",
            nodes.iter().map(|n| &n.kind).collect::<Vec<_>>()
        );
    }

    #[test]
    fn parses_typescript_function() {
        let source = "function hello(a: number): void { return; }";
        let mut nodes = parse_file(source, Language::TypeScript, SyntaxDepth::Statement).unwrap();
        build_child_links(&mut nodes);
        assert!(
            nodes.iter().any(|n| n.kind == "function_declaration"),
            "expected function_declaration node, got kinds: {:?}",
            nodes.iter().map(|n| &n.kind).collect::<Vec<_>>()
        );
    }

    #[test]
    fn byte_offsets_are_correct() {
        let source = "fn hello() { }";
        let nodes = parse_file(source, Language::Rust, SyntaxDepth::Statement).unwrap();
        let func = nodes
            .iter()
            .find(|n| n.kind == "function_item")
            .unwrap();
        assert!(func.start_byte < func.end_byte);
        assert_eq!(&source[func.start_byte..func.end_byte], "fn hello() { }");
    }

    #[test]
    fn whitespace_hash_is_stable() {
        let a = whitespace_normalized_hash("fn  hello ( ) ");
        let b = whitespace_normalized_hash("fn hello()");
        assert_eq!(a, b);
    }

    #[test]
    fn empty_source_parses() {
        let mut nodes = parse_file("", Language::Rust, SyntaxDepth::Statement).unwrap();
        build_child_links(&mut nodes);
        assert!(nodes.is_empty() || nodes.iter().all(|n| n.kind == "source_file"));
    }

    #[test]
    fn multiple_functions_found() {
        let source = "fn a() { }\nfn b() { }\n";
        let mut nodes = parse_file(source, Language::Rust, SyntaxDepth::Statement).unwrap();
        build_child_links(&mut nodes);
        let count = nodes
            .iter()
            .filter(|n| n.kind == "function_item")
            .count();
        assert_eq!(count, 2, "expected 2 function items");
    }

    #[test]
    fn parent_child_links_built() {
        let source = "fn foo() { fn bar() {} }";
        let mut nodes = parse_file(source, Language::Rust, SyntaxDepth::Statement).unwrap();
        build_child_links(&mut nodes);
        // With source_file excluded, function_item for foo has no parent.
        let foo = nodes.iter().find(|n| n.kind == "function_item" && n.text == "fn foo() { fn bar() {} }").unwrap();
        assert!(foo.parent.is_none(), "foo should be top-level");
        // Nested function_item bar should have a parent (the block node).
        let bar = nodes.iter().find(|n| n.kind == "function_item" && n.text == "fn bar() {}");
        assert!(bar.is_some(), "expected nested function_item bar");
        assert!(bar.unwrap().parent.is_some(), "bar should have a parent");
    }

    #[test]
    fn line_numbers_for_multiline() {
        let source = "fn a() {\n    1\n}\n";
        let nodes = parse_file(source, Language::Rust, SyntaxDepth::Statement).unwrap();
        let func = nodes
            .iter()
            .find(|n| n.kind == "function_item")
            .unwrap();
        assert_eq!(func.start_line, 1);
        assert_eq!(func.end_line, 3);
    }
}
