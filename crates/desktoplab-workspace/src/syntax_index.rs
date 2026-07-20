use std::path::Path;
use tree_sitter::{Language, Node, Parser};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeSymbol {
    name: String,
    kind: String,
    line: usize,
}

impl CodeSymbol {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    #[must_use]
    pub fn line(&self) -> usize {
        self.line
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeReference {
    name: String,
    line: usize,
}

impl CodeReference {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn line(&self) -> usize {
        self.line
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SyntaxIndex {
    symbols: Vec<CodeSymbol>,
    references: Vec<CodeReference>,
    dependency_hints: Vec<String>,
}

impl SyntaxIndex {
    #[must_use]
    pub fn symbols(&self) -> &[CodeSymbol] {
        &self.symbols
    }

    #[must_use]
    pub fn references(&self) -> &[CodeReference] {
        &self.references
    }

    #[must_use]
    pub fn dependency_hints(&self) -> &[String] {
        &self.dependency_hints
    }
}

pub fn parse_syntax(path: &Path, source: &str) -> SyntaxIndex {
    let Some(language) = language_for_path(path) else {
        return SyntaxIndex::default();
    };
    let mut parser = Parser::new();
    if parser.set_language(&language).is_err() {
        return SyntaxIndex::default();
    }
    let Some(tree) = parser.parse(source, None) else {
        return SyntaxIndex::default();
    };
    let mut index = SyntaxIndex::default();
    collect_node(tree.root_node(), source, &mut index, &[]);
    index
}

fn collect_node(
    node: Node<'_>,
    source: &str,
    index: &mut SyntaxIndex,
    definition_ranges: &[(usize, usize)],
) {
    let mut ranges = definition_ranges.to_vec();
    if let Some(kind) = definition_kind(node.kind())
        && let Some(name_node) = node.child_by_field_name("name")
        && let Ok(name) = name_node.utf8_text(source.as_bytes())
    {
        index.symbols.push(CodeSymbol {
            name: name.to_string(),
            kind: kind.to_string(),
            line: name_node.start_position().row + 1,
        });
        ranges.push((name_node.start_byte(), name_node.end_byte()));
    }
    if is_dependency_node(node.kind())
        && let Ok(text) = node.utf8_text(source.as_bytes())
    {
        index.dependency_hints.push(compact(text));
    }
    if is_reference_node(node.kind())
        && !ranges
            .iter()
            .any(|(start, end)| node.start_byte() >= *start && node.end_byte() <= *end)
        && let Ok(name) = node.utf8_text(source.as_bytes())
    {
        index.references.push(CodeReference {
            name: name.to_string(),
            line: node.start_position().row + 1,
        });
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_node(child, source, index, &ranges);
    }
}

fn language_for_path(path: &Path) -> Option<Language> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("rs") => Some(tree_sitter_rust::LANGUAGE.into()),
        Some("ts") => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        Some("tsx") => Some(tree_sitter_typescript::LANGUAGE_TSX.into()),
        Some("py") => Some(tree_sitter_python::LANGUAGE.into()),
        _ => None,
    }
}

fn definition_kind(kind: &str) -> Option<&'static str> {
    match kind {
        "function_item" | "function_declaration" | "function_definition" => Some("function"),
        "struct_item" => Some("struct"),
        "enum_item" => Some("enum"),
        "trait_item" => Some("trait"),
        "class_declaration" | "class_definition" => Some("class"),
        "interface_declaration" => Some("interface"),
        "type_alias_declaration" | "type_item" => Some("type"),
        "variable_declarator" => Some("variable"),
        "const_item" | "static_item" => Some("constant"),
        "mod_item" => Some("module"),
        _ => None,
    }
}

fn is_reference_node(kind: &str) -> bool {
    matches!(kind, "identifier" | "type_identifier" | "field_identifier")
}

fn is_dependency_node(kind: &str) -> bool {
    matches!(
        kind,
        "use_declaration"
            | "import_statement"
            | "export_statement"
            | "import_from_statement"
            | "future_import_statement"
    )
}

fn compact(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}
