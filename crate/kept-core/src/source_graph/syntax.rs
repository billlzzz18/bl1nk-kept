//! Tree-sitter AST symbol extraction สำหรับ Rust source
//! NOTE-P1.2: parse_rust_ast ย้ายไปใช้ Query API + rust.scm แทน node-walk
//! parse_rust_outline ยังคง node-walk เพราะต้องการ parent-child tree structure

use super::{
    DefinitionKind, ImplementationRecord, ImportRecord, ReferenceKind, ScopeGraphIndex,
    SymbolDefinition, SymbolReference,
};
use crate::observation::StructureOutlineItem;
use streaming_iterator::StreamingIterator;
use tree_sitter::{Node, Parser, Query, QueryCursor, Tree, TreeCursor};

const RUST_QUERY: &str = include_str!("../parsers/rust.scm");

/// สร้าง parser สำหรับ Rust (คืน None ถ้า set language ไม่สำเร็จ)
fn rust_parser() -> Option<Parser> {
    let mut parser = Parser::new();
    let language = tree_sitter_rust::LANGUAGE;
    if parser.set_language(&language.into()).is_err() {
        return None;
    }
    Some(parser)
}

/// Parse Rust source เป็น AST แล้ว extract symbols ลง index ด้วย Query API
/// คืน None ถ้า parser หรือ query ใช้ไม่ได้
pub fn parse_rust_ast(path_str: &str, content: &str) -> Option<ScopeGraphIndex> {
    let mut parser = rust_parser()?;
    let tree = parser.parse(content, None)?;
    let lang = tree_sitter_rust::LANGUAGE.into();
    let query = match Query::new(&lang, RUST_QUERY) {
        Ok(q) => q,
        Err(e) => {
            eprintln!("Query::new failed: {:?}", e);
            return None;
        },
    };

    let mut index = ScopeGraphIndex::new();
    index.indexed_files.insert(path_str.to_string());

    let mut cursor = QueryCursor::new();
    let source_bytes = content.as_bytes();

    // NOTE-023: tree-sitter 0.25 QueryMatches ใช้ StreamingIterator ไม่ใช่ std::Iterator
    let mut matches = cursor.matches(&query, tree.root_node(), source_bytes);
    while let Some(mat) = {
        matches.advance();
        matches.get()
    } {
        for capture in mat.captures {
            let capture_name = &query.capture_names()[capture.index as usize];
            let node = capture.node;

            match &**capture_name {
                "definition.function" => {
                    if let Some(name) = extract_name(node, source_bytes) {
                        index.definitions.push(SymbolDefinition {
                            name,
                            kind: DefinitionKind::Function,
                            file_path: path_str.to_string(),
                            line: node.start_position().row + 1,
                            column: node.start_position().column + 1,
                            doc: preceding_doc_bytes(node, source_bytes),
                        });
                    }
                },
                "definition.struct" => {
                    if let Some(name) = extract_name(node, source_bytes) {
                        index.definitions.push(SymbolDefinition {
                            name,
                            kind: DefinitionKind::Struct,
                            file_path: path_str.to_string(),
                            line: node.start_position().row + 1,
                            column: node.start_position().column + 1,
                            doc: preceding_doc_bytes(node, source_bytes),
                        });
                    }
                },
                "definition.enum" => {
                    if let Some(name) = extract_name(node, source_bytes) {
                        index.definitions.push(SymbolDefinition {
                            name,
                            kind: DefinitionKind::Enum,
                            file_path: path_str.to_string(),
                            line: node.start_position().row + 1,
                            column: node.start_position().column + 1,
                            doc: preceding_doc_bytes(node, source_bytes),
                        });
                    }
                },
                "definition.trait" => {
                    if let Some(name) = extract_name(node, source_bytes) {
                        index.definitions.push(SymbolDefinition {
                            name,
                            kind: DefinitionKind::Trait,
                            file_path: path_str.to_string(),
                            line: node.start_position().row + 1,
                            column: node.start_position().column + 1,
                            doc: preceding_doc_bytes(node, source_bytes),
                        });
                    }
                },
                "definition.type_alias" => {
                    if let Some(name) = extract_name(node, source_bytes) {
                        index.definitions.push(SymbolDefinition {
                            name,
                            kind: DefinitionKind::TypeAlias,
                            file_path: path_str.to_string(),
                            line: node.start_position().row + 1,
                            column: node.start_position().column + 1,
                            doc: preceding_doc_bytes(node, source_bytes),
                        });
                    }
                },
                "definition.macro" => {
                    if let Some(name) = extract_name(node, source_bytes) {
                        index.definitions.push(SymbolDefinition {
                            name,
                            kind: DefinitionKind::Other("macro".to_string()),
                            file_path: path_str.to_string(),
                            line: node.start_position().row + 1,
                            column: node.start_position().column + 1,
                            doc: preceding_doc_bytes(node, source_bytes),
                        });
                    }
                },
                "implementation.block" => {
                    // Parse the entire impl block text to extract trait and target type
                    let text = node_text_bytes(node, source_bytes);
                    // Format: "impl<T: Clone> Visitor<'static> for Container<T>" or "impl Container<u8>"
                    let body = text.strip_prefix("impl").unwrap_or(text).trim();
                    // Find " for " to separate trait and target
                    if let Some(for_idx) = body.find(" for ") {
                        let trait_part = body[..for_idx].trim();
                        let target_part = body[for_idx + 5..].trim();
                        // Extract trait name: find the identifier after the generic parameters
                        // e.g., "<T: Clone> Visitor<'static>" → "Visitor"
                        // The generic parameters end at the first '>' that's not inside another '<>'
                        let trait_name = if trait_part.starts_with('<') {
                            // Find the matching '>' for the opening '<'
                            let mut depth = 0;
                            let mut end_of_generics = 0;
                            for (i, c) in trait_part.char_indices() {
                                match c {
                                    '<' => depth += 1,
                                    '>' => {
                                        depth -= 1;
                                        if depth == 0 {
                                            end_of_generics = i + 1;
                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            let after_generics = trait_part[end_of_generics..].trim();
                            after_generics
                                .split('<')
                                .next()
                                .unwrap_or(after_generics)
                                .trim()
                                .to_string()
                        } else {
                            // No generics — the whole thing is the trait name
                            trait_part
                                .split('<')
                                .next()
                                .unwrap_or(trait_part)
                                .trim()
                                .to_string()
                        };
                        let target_type = target_part
                            .split('<')
                            .next()
                            .unwrap_or(target_part)
                            .split('{')
                            .next()
                            .unwrap_or(target_part)
                            .trim()
                            .to_string();
                        if !target_type.is_empty() {
                            index.implementations.push(ImplementationRecord {
                                trait_name: Some(trait_name),
                                target_type,
                                file_path: path_str.to_string(),
                                line: node.start_position().row + 1,
                            });
                        }
                    } else {
                        // No " for " — this is an inherent impl
                        let target_type = body
                            .split('<')
                            .next()
                            .unwrap_or(body)
                            .split('{')
                            .next()
                            .unwrap_or(body)
                            .trim()
                            .to_string();
                        if !target_type.is_empty() {
                            index.implementations.push(ImplementationRecord {
                                trait_name: None,
                                target_type,
                                file_path: path_str.to_string(),
                                line: node.start_position().row + 1,
                            });
                        }
                    }
                },
                "import.statement" => {
                    // Parse the entire use_declaration node text
                    // Format: "use path::to::module;" or "use path::to::module as alias;" or "use path::to::*;"
                    let raw = node_text_bytes(node, source_bytes);
                    let stripped = raw
                        .strip_prefix("use ")
                        .unwrap_or(raw)
                        .trim_end_matches(';')
                        .trim();
                    let is_wildcard = stripped.ends_with("::*");
                    let (path_part, alias) = match stripped.find(" as ") {
                        Some(idx) => (
                            stripped[..idx].trim().to_string(),
                            Some(stripped[idx + 4..].trim().to_string()),
                        ),
                        None => (stripped.to_string(), None),
                    };
                    index.imports.push(ImportRecord {
                        path: path_part,
                        alias,
                        file_path: path_str.to_string(),
                        line: node.start_position().row + 1,
                        is_wildcard,
                    });
                },
                "reference.call" => {
                    let text = node_text_bytes(node, source_bytes);
                    let name = text.rsplit([':', '.']).next().unwrap_or(text).trim();
                    if !name.is_empty() {
                        index.references.push(SymbolReference {
                            symbol_name: name.to_string(),
                            kind: ReferenceKind::Call,
                            file_path: path_str.to_string(),
                            line: node.start_position().row + 1,
                            column: node.start_position().column + 1,
                        });
                    }
                },
                "reference.type" => {
                    let name = node_text_bytes(node, source_bytes).to_string();
                    if !name.is_empty() {
                        index.references.push(SymbolReference {
                            symbol_name: name,
                            kind: ReferenceKind::TypeUsage,
                            file_path: path_str.to_string(),
                            line: node.start_position().row + 1,
                            column: node.start_position().column + 1,
                        });
                    }
                },
                "scope" => {}, // ใช้สำหรับ boundary เท่านั้น ไม่จำเป็นต้อง capture
                _ => {},
            }
        }
    }

    Some(index)
}

/// Parse Rust source เป็น structural outline (สำหรับ `StructurePayload` ของ Observation)
/// คืน None ถ้า parser ใช้ไม่ได้ — ยังคงใช้ node-walk เพราะ outline ต้องการ tree structure
pub fn parse_rust_outline(content: &str) -> Option<Vec<StructureOutlineItem>> {
    let mut parser = rust_parser()?;
    let tree: Tree = parser.parse(content, None)?;
    let mut cursor = tree.walk();
    Some(collect_outline(&mut cursor, content))
}

/// ข้อความ UTF-8 จาก node (byte slice)
fn node_text_bytes<'a>(node: Node<'a>, source_bytes: &'a [u8]) -> &'a str {
    std::str::from_utf8(&source_bytes[node.start_byte()..node.end_byte()]).unwrap_or("")
}

/// ชื่อ definition จาก field \"name\" ของ item node (byte API)
fn extract_name(node: Node, source_bytes: &[u8]) -> Option<String> {
    // node ใน query จะชี้ที่ identifier node โดยตรง (ไม่ใช่ parent item)
    let text = node_text_bytes(node, source_bytes).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// ข้อความของ node จาก source
fn node_text<'a>(node: Node<'a>, content: &'a str) -> &'a str {
    &content[node.start_byte()..node.end_byte()]
}

/// ตัด generic/attribute ท้ายชื่อ type เหลือ base name เช่น `Container<T>` → `Container`
fn base_type_name(text: &str) -> String {
    text.split('<').next().unwrap_or(text).trim().to_string()
}

/// ชื่อ definition จาก field "name" ของ item node
fn item_name(node: Node, content: &str) -> Option<String> {
    node.child_by_field_name("name")
        .map(|name| node_text(name, content).to_string())
}

/// ชื่อสำหรับ outline — impl_item ไม่มี field "name" จึงใช้ field "type" แทน
fn outline_name(node: Node, content: &str) -> Option<String> {
    item_name(node, content).or_else(|| {
        if node.kind() == "impl_item" {
            node.child_by_field_name("type")
                .map(|t| base_type_name(node_text(t, content)))
        } else {
            None
        }
    })
}

/// doc comment (`///`) ของ item จาก named sibling ก่อนหน้า (byte version สำหรับ Query API)
fn preceding_doc_bytes(node: Node, source_bytes: &[u8]) -> Option<String> {
    let mut sibling = node;
    while let Some(prev) = sibling.prev_named_sibling() {
        if prev.kind() != "line_comment" {
            return None;
        }
        let text = node_text_bytes(prev, source_bytes);
        if text.starts_with("///") {
            return Some(text.trim_start_matches('/').trim().to_string());
        }
        sibling = prev;
    }
    None
}

/// สร้าง outline จาก items ใต้ cursor ปัจจุบัน (children = nested items)
fn collect_outline(cursor: &mut TreeCursor, content: &str) -> Vec<StructureOutlineItem> {
    let mut outline = Vec::new();
    loop {
        let node = cursor.node();
        if let Some(kind_name) = outline_kind(node.kind()) {
            if let Some(name) = outline_name(node, content) {
                // เก็บ children โดยลงไปใน body ของ item นี้เท่านั้น
                let children = if cursor.goto_first_child() {
                    let nested = collect_outline(cursor, content);
                    cursor.goto_parent();
                    nested
                } else {
                    Vec::new()
                };
                outline.push(StructureOutlineItem {
                    name,
                    kind: kind_name.to_string(),
                    range: Some((node.start_position().row + 1, node.end_position().row + 1)),
                    children,
                });
            }
        } else if cursor.goto_first_child() {
            // node ไม่ใช่ item — ไต่ลงไปหา item ข้างใน (เช่น declaration_list)
            let nested = collect_outline(cursor, content);
            cursor.goto_parent();
            outline.extend(nested);
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    outline
}

/// map node kind → ชื่อ kind ใน outline
fn outline_kind(kind: &str) -> Option<&'static str> {
    match kind {
        "function_item" | "function_signature_item" => Some("function"),
        "struct_item" => Some("struct"),
        "enum_item" => Some("enum"),
        "trait_item" => Some("trait"),
        "type_item" => Some("type_alias"),
        "mod_item" => Some("module"),
        "impl_item" => Some("impl"),
        "macro_definition" => Some("macro"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outline_captures_top_level_and_children() {
        // outline ต้องได้ top-level items พร้อม children เป็น method ภายใน
        let code = r#"
            pub struct Wallet {
                balance: u64,
            }

            impl Wallet {
                pub fn deposit(&mut self, amount: u64) {}
                pub fn balance(&self) -> u64 { self.balance }
            }
        "#;
        let outline = parse_rust_outline(code).unwrap_or_default();
        let names: Vec<&str> = outline.iter().map(|item| item.name.as_str()).collect();
        assert!(names.contains(&"Wallet"));
        // impl block เป็น top-level item และมี method เป็น children
        let impl_item = outline
            .iter()
            .find(|item| item.kind == "impl" && item.name == "Wallet")
            .unwrap_or_else(|| panic!("impl Wallet must be in outline"));
        let method_names: Vec<&str> = impl_item.children.iter().map(|c| c.name.as_str()).collect();
        assert!(method_names.contains(&"deposit"));
        assert!(method_names.contains(&"balance"));
        // range ต้องเป็น (start_line, end_line) แบบ 1-based
        let struct_item = &outline[0];
        assert_eq!(struct_item.range, Some((2, 4)));
    }
}
