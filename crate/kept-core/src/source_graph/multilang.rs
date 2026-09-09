//! Multi-language tree-sitter symbol extraction (Python, JS/TS, Go)
//! NOTE-P1.1b: ต่อยอดจาก Rust extraction — เลือก parser จากนามสกุลไฟล์
//! แล้ว walk AST ด้วย node-kind mapping ต่อภาษา

use super::{DefinitionKind, ImportRecord, ScopeGraphIndex, SymbolDefinition};
use tree_sitter::{Node, Parser, TreeCursor};

/// ภาษาที่ parser รองรับ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
}

/// เลือกภาษาจากนามสกุลไฟล์
pub fn language_from_extension(extension: &str) -> Option<Language> {
    match extension {
        "rs" => Some(Language::Rust),
        "py" => Some(Language::Python),
        "js" | "jsx" | "mjs" | "cjs" => Some(Language::JavaScript),
        "ts" | "tsx" | "mts" | "cts" => Some(Language::TypeScript),
        "go" => Some(Language::Go),
        _ => None,
    }
}

/// สร้าง parser สำหรับภาษาที่กำหนด
fn parser_for(language: Language) -> Option<Parser> {
    let mut parser = Parser::new();
    let language_ref: tree_sitter::Language = match language {
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        Language::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::Go => tree_sitter_go::LANGUAGE.into(),
    };
    if parser.set_language(&language_ref).is_err() {
        return None;
    }
    Some(parser)
}

/// ข้อความของ node จาก source
fn node_text<'a>(node: Node<'a>, content: &'a str) -> &'a str {
    &content[node.start_byte()..node.end_byte()]
}

/// ชื่อจาก field "name"
fn field_name(node: Node, content: &str) -> Option<String> {
    node.child_by_field_name("name")
        .map(|name| node_text(name, content).to_string())
}

/// บันทึก definition
fn push_definition(
    index: &mut ScopeGraphIndex,
    node: Node,
    name: String,
    kind: DefinitionKind,
    path_str: &str,
) {
    let position = node
        .child_by_field_name("name")
        .unwrap_or(node)
        .start_position();
    index.definitions.push(SymbolDefinition {
        name,
        kind,
        file_path: path_str.to_string(),
        line: position.row + 1,
        column: position.column + 1,
        doc: None,
    });
}

/// บันทึก import record
fn push_import(index: &mut ScopeGraphIndex, path: String, path_str: &str, node: Node) {
    index.imports.push(ImportRecord {
        path,
        alias: None,
        file_path: path_str.to_string(),
        line: node.start_position().row + 1,
        is_wildcard: false,
    });
}

/// Parse source code ตามภาษา แล้ว extract symbols ลง index
pub fn parse_source(language: Language, path_str: &str, content: &str) -> Option<ScopeGraphIndex> {
    let mut parser = parser_for(language)?;
    let tree = parser.parse(content, None)?;
    let mut index = ScopeGraphIndex::new();
    index.indexed_files.insert(path_str.to_string());

    let mut cursor = tree.walk();
    walk_multilang(&mut cursor, content, path_str, language, &mut index);
    Some(index)
}

/// walk AST ของภาษา non-Rust — จับ definitions และ imports ตาม node kind
fn walk_multilang(
    cursor: &mut TreeCursor,
    content: &str,
    path_str: &str,
    language: Language,
    index: &mut ScopeGraphIndex,
) {
    loop {
        let node = cursor.node();
        let kind = node.kind();

        match language {
            Language::Python => match kind {
                "function_definition" => {
                    if let Some(name) = field_name(node, content) {
                        push_definition(index, node, name, DefinitionKind::Function, path_str);
                    }
                }
                "class_definition" => {
                    if let Some(name) = field_name(node, content) {
                        // NOTE-011: map Python class → Struct (ตาม convention ของ graph)
                        push_definition(index, node, name, DefinitionKind::Struct, path_str);
                    }
                }
                "import_statement" => {
                    // import os / import os, sys — จับทุก dotted_name / aliased_import
                    let mut child_cursor = node.walk();
                    for child in node.children(&mut child_cursor) {
                        if matches!(child.kind(), "dotted_name" | "aliased_import") {
                            let text = node_text(child, content);
                            let module_name = text.split(" as ").next().unwrap_or(text);
                            push_import(index, module_name.trim().to_string(), path_str, node);
                        }
                    }
                }
                "import_from_statement" => {
                    // from x import y — module_name field
                    if let Some(module) = node.child_by_field_name("module_name") {
                        push_import(
                            index,
                            node_text(module, content).to_string(),
                            path_str,
                            node,
                        );
                    }
                }
                _ => {}
            },
            Language::JavaScript | Language::TypeScript => match kind {
                "function_declaration" | "generator_function_declaration" => {
                    if let Some(name) = field_name(node, content) {
                        push_definition(index, node, name, DefinitionKind::Function, path_str);
                    }
                }
                "class_declaration" => {
                    if let Some(name) = field_name(node, content) {
                        push_definition(index, node, name, DefinitionKind::Struct, path_str);
                    }
                }
                "method_definition" => {
                    if let Some(name) = field_name(node, content) {
                        push_definition(index, node, name, DefinitionKind::Function, path_str);
                    }
                }
                "interface_declaration" => {
                    if let Some(name) = field_name(node, content) {
                        push_definition(index, node, name, DefinitionKind::Trait, path_str);
                    }
                }
                "type_alias_declaration" => {
                    if let Some(name) = field_name(node, content) {
                        push_definition(index, node, name, DefinitionKind::TypeAlias, path_str);
                    }
                }
                "enum_declaration" => {
                    if let Some(name) = field_name(node, content) {
                        push_definition(index, node, name, DefinitionKind::Enum, path_str);
                    }
                }
                "import_statement" => {
                    // import ... from 'source' — ใช้ค่า string ของ source
                    if let Some(source) = node.child_by_field_name("source") {
                        let raw = node_text(source, content);
                        let module_path = raw.trim_matches(|c| c == '\'' || c == '"');
                        push_import(index, module_path.to_string(), path_str, node);
                    }
                }
                _ => {}
            },
            Language::Go => match kind {
                "function_declaration" => {
                    if let Some(name) = field_name(node, content) {
                        push_definition(index, node, name, DefinitionKind::Function, path_str);
                    }
                }
                "method_declaration" => {
                    if let Some(name) = field_name(node, content) {
                        push_definition(index, node, name, DefinitionKind::Function, path_str);
                    }
                }
                "type_spec" => {
                    // type X struct {...} | interface {...} | alias
                    if let Some(name) = field_name(node, content) {
                        let kind = match node.child(1).map(|c| c.kind()) {
                            Some("struct_type") => DefinitionKind::Struct,
                            Some("interface_type") => DefinitionKind::Trait,
                            _ => DefinitionKind::TypeAlias,
                        };
                        push_definition(index, node, name, kind, path_str);
                    }
                }
                "import_spec" => {
                    // import "fmt" หรือ import alias "path"
                    if let Some(path_node) = node.child_by_field_name("path") {
                        let raw = node_text(path_node, content);
                        push_import(index, raw.trim_matches('"').to_string(), path_str, node);
                    }
                }
                _ => {}
            },
            Language::Rust => {}
        }

        // ลงลึก children (nested defs เช่น method ใน class)
        if cursor.goto_first_child() {
            walk_multilang(cursor, content, path_str, language, index);
            cursor.goto_parent();
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
}
