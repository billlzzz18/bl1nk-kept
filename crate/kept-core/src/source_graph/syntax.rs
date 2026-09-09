//! Tree-sitter AST symbol extraction สำหรับ Rust source
//! NOTE-P1.1: แทน regex string matching ใน `parse_rust_file` เพื่อความถูกต้อง
//! (ไม่หลอกด้วย string literal/comment, รองรับ multi-line signature, generics, macro)

use super::{
    DefinitionKind, ImplementationRecord, ImportRecord, ReferenceKind, ScopeGraphIndex,
    SymbolDefinition, SymbolReference,
};
use crate::observation::StructureOutlineItem;
use tree_sitter::{Node, Parser, Tree, TreeCursor};

/// สร้าง parser สำหรับ Rust (คืน None ถ้า set language ไม่สำเร็จ)
fn rust_parser() -> Option<Parser> {
    let mut parser = Parser::new();
    let language = tree_sitter_rust::LANGUAGE;
    if parser.set_language(&language.into()).is_err() {
        return None;
    }
    Some(parser)
}

/// Parse Rust source เป็น AST แล้ว extract symbols ลง index
/// คืน None ถ้า parser ใช้ไม่ได้ (caller ต้อง fallback)
pub fn parse_rust_ast(path_str: &str, content: &str) -> Option<ScopeGraphIndex> {
    let mut parser = rust_parser()?;
    let tree = parser.parse(content, None)?;
    let mut index = ScopeGraphIndex::new();
    index.indexed_files.insert(path_str.to_string());

    let mut cursor = tree.walk();
    walk_node(&mut cursor, content, path_str, &mut index);
    Some(index)
}

/// Parse Rust source เป็น structural outline (สำหรับ `StructurePayload` ของ Observation)
/// คืน None ถ้า parser ใช้ไม่ได้
pub fn parse_rust_outline(content: &str) -> Option<Vec<StructureOutlineItem>> {
    let mut parser = rust_parser()?;
    let tree: Tree = parser.parse(content, None)?;
    let mut cursor = tree.walk();
    Some(collect_outline(&mut cursor, content))
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

/// doc comment (`///`) ของ item จาก named sibling ก่อนหน้า
fn preceding_doc(node: Node, content: &str) -> Option<String> {
    let mut sibling = node;
    while let Some(prev) = sibling.prev_named_sibling() {
        if prev.kind() != "line_comment" {
            return None;
        }
        let text = node_text(prev, content);
        if text.starts_with("///") {
            return Some(text.trim_start_matches('/').trim().to_string());
        }
        sibling = prev;
    }
    None
}

/// บันทึก definition ลง index
fn push_definition(
    index: &mut ScopeGraphIndex,
    node: Node,
    name: String,
    kind: DefinitionKind,
    path_str: &str,
    content: &str,
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
        doc: preceding_doc(node, content),
    });
}

/// walk ทุก node แบบ recursive — เก็บ definitions, imports, impls, references
fn walk_node(cursor: &mut TreeCursor, content: &str, path_str: &str, index: &mut ScopeGraphIndex) {
    loop {
        let node = cursor.node();
        match node.kind() {
            // NOTE-010: tree-sitter-rust ใช้ kind "function_item" (มี body)
            // และ "function_signature_item" (method ใน trait ที่ไม่มี body)
            "function_item" | "function_signature_item" => {
                if let Some(name) = item_name(node, content) {
                    push_definition(
                        index,
                        node,
                        name,
                        DefinitionKind::Function,
                        path_str,
                        content,
                    );
                }
            }
            "struct_item" => {
                if let Some(name) = item_name(node, content) {
                    push_definition(index, node, name, DefinitionKind::Struct, path_str, content);
                }
            }
            "enum_item" => {
                if let Some(name) = item_name(node, content) {
                    push_definition(index, node, name, DefinitionKind::Enum, path_str, content);
                }
            }
            "trait_item" => {
                if let Some(name) = item_name(node, content) {
                    push_definition(index, node, name, DefinitionKind::Trait, path_str, content);
                }
            }
            "type_item" => {
                if let Some(name) = item_name(node, content) {
                    push_definition(
                        index,
                        node,
                        name,
                        DefinitionKind::TypeAlias,
                        path_str,
                        content,
                    );
                }
            }
            "mod_item" => {
                if let Some(name) = item_name(node, content) {
                    push_definition(index, node, name, DefinitionKind::Module, path_str, content);
                }
            }
            "macro_definition" => {
                if let Some(name) = item_name(node, content) {
                    push_definition(
                        index,
                        node,
                        name,
                        DefinitionKind::Other("macro".to_string()),
                        path_str,
                        content,
                    );
                }
            }
            "impl_item" => {
                // NOTE-004: impl Trait for Type หรือ impl Type — ใช้ base name ตัด generics
                let trait_name = node
                    .child_by_field_name("trait")
                    .map(|t| base_type_name(node_text(t, content)));
                let target_type = node
                    .child_by_field_name("type")
                    .map(|t| base_type_name(node_text(t, content)))
                    .unwrap_or_default();
                if !target_type.is_empty() {
                    index.implementations.push(ImplementationRecord {
                        trait_name,
                        target_type,
                        file_path: path_str.to_string(),
                        line: node.start_position().row + 1,
                    });
                }
            }
            "use_declaration" => {
                // NOTE-005: ตัด "use " หัว และ ";" ท้ายจาก raw text ของ declaration
                let raw = node_text(node, content).trim();
                let body = raw
                    .strip_prefix("use ")
                    .unwrap_or(raw)
                    .trim_end_matches(';')
                    .trim();
                if !body.is_empty() {
                    let is_wildcard = body.ends_with("::*");
                    let (path_part, alias) = match body.find(" as ") {
                        Some(idx) => (
                            body[..idx].trim().to_string(),
                            Some(body[idx + 4..].trim().to_string()),
                        ),
                        None => (body.to_string(), None),
                    };
                    index.imports.push(ImportRecord {
                        path: path_part,
                        alias,
                        file_path: path_str.to_string(),
                        line: node.start_position().row + 1,
                        is_wildcard,
                    });
                }
            }
            "call_expression" => {
                // เก็บชื่อ function ที่ถูกเรียกเป็น Call reference
                if let Some(function) = node.child_by_field_name("function") {
                    if matches!(
                        function.kind(),
                        "identifier" | "field_expression" | "scoped_identifier"
                    ) {
                        // scoped/field: ใช้ segment สุดท้ายเป็นชื่อ
                        let text = node_text(function, content);
                        let name = text.rsplit([':', '.']).next().unwrap_or(text).trim();
                        if !name.is_empty() {
                            index.references.push(SymbolReference {
                                symbol_name: name.to_string(),
                                kind: ReferenceKind::Call,
                                file_path: path_str.to_string(),
                                line: function.start_position().row + 1,
                                column: function.start_position().column + 1,
                            });
                        }
                    }
                }
            }
            "type_identifier" => {
                // NOTE-006: การอ้างอิง type ทุกจุด = TypeUsage reference
                index.references.push(SymbolReference {
                    symbol_name: node_text(node, content).to_string(),
                    kind: ReferenceKind::TypeUsage,
                    file_path: path_str.to_string(),
                    line: node.start_position().row + 1,
                    column: node.start_position().column + 1,
                });
            }
            _ => {}
        }

        // NOTE-007: ลงลึก children เพื่อจับ nested items (method ใน impl/trait, fn ใน mod)
        if cursor.goto_first_child() {
            walk_node(cursor, content, path_str, index);
            cursor.goto_parent();
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
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
