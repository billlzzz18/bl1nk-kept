//! Phase 1.1: Tree-sitter AST symbol extraction contract tests
//! ทดสอบ behavior ที่ regex parser ทำไม่ได้: string literal, comment,
//! multi-line signature, generics, macro, call references

use kept_core::source_graph::{DefinitionKind, IndexBuilder, ReferenceKind};
use std::path::Path;

#[test]
fn string_literal_must_not_produce_definition() {
    // "fn fake()" ใน string literal ต้องไม่ถูกนับเป็น function definition
    let code = r#"
        fn real_function() {
            let message = "fn fake() inside string";
        }
    "#;
    let index = IndexBuilder::parse_rust_file(Path::new("a.rs"), code);
    let defs = index.find_definitions("fake");
    assert!(defs.is_empty(), "fake from string literal must not be a definition");
    assert_eq!(index.find_definitions("real_function").len(), 1);
}

#[test]
fn comment_must_not_produce_definition() {
    // "fn ghost()" ใน comment ต้องไม่ถูกนับเป็น definition
    let code = r#"
        // fn ghost() in a comment
        /* fn block_ghost() too */
        fn visible() {}
    "#;
    let index = IndexBuilder::parse_rust_file(Path::new("a.rs"), code);
    assert!(index.find_definitions("ghost").is_empty());
    assert!(index.find_definitions("block_ghost").is_empty());
    assert_eq!(index.find_definitions("visible").len(), 1);
}

#[test]
fn multiline_signature_captured_once() {
    // signature ข้ามหลายบรรทัดต้อง capture ครั้งเดียวที่บรรทัดของ fn
    let code = r#"
        pub fn compute_value(
            first: u64,
            second: u64,
        ) -> u64 {
            first + second
        }
    "#;
    let index = IndexBuilder::parse_rust_file(Path::new("a.rs"), code);
    let defs = index.find_definitions("compute_value");
    assert_eq!(defs.len(), 1, "multi-line signature must yield exactly one definition");
    assert_eq!(defs[0].kind, DefinitionKind::Function);
    assert_eq!(defs[0].line, 2);
}

#[test]
fn generics_struct_enum_trait_captured() {
    let code = r#"
        pub struct Container<T> {
            inner: T,
        }

        pub enum Outcome<T, E> {
            Ok(T),
            Err(E),
        }

        pub trait Visitor<'a> {
            fn visit(&mut self);
        }
    "#;
    let index = IndexBuilder::parse_rust_file(Path::new("a.rs"), code);
    assert_eq!(index.find_definitions("Container").len(), 1);
    assert_eq!(index.find_definitions("Outcome").len(), 1);
    assert_eq!(index.find_definitions("Visitor").len(), 1);
    // method ใน trait ก็ต้องถูก capture
    assert_eq!(index.find_definitions("visit").len(), 1);
}

#[test]
fn impl_generic_trait_for_type() {
    let code = r#"
        impl<T: Clone> Visitor<'static> for Container<T> {
            fn visit(&mut self) {}
        }

        impl Container<u8> {
            fn capacity(&self) -> usize { 0 }
        }
    "#;
    let index = IndexBuilder::parse_rust_file(Path::new("a.rs"), code);
    assert_eq!(index.implementations.len(), 2);
    assert_eq!(index.implementations[0].trait_name, Some("Visitor".to_string()));
    assert_eq!(index.implementations[0].target_type, "Container");
    assert_eq!(index.implementations[1].trait_name, None);
    assert_eq!(index.implementations[1].target_type, "Container");
}

#[test]
fn macro_rules_definition_captured() {
    let code = r#"
        macro_rules! make_vec {
            () => { Vec::new() };
        }
    "#;
    let index = IndexBuilder::parse_rust_file(Path::new("a.rs"), code);
    let defs = index.find_definitions("make_vec");
    assert_eq!(defs.len(), 1, "macro_rules! name must be a definition");
}

#[test]
fn call_expression_produces_call_reference() {
    let code = r#"
        fn main() {
            let value = helper(1);
            let doubled = helper(value);
        }
    "#;
    let index = IndexBuilder::parse_rust_file(Path::new("a.rs"), code);
    let refs = index.find_references("helper");
    assert!(refs.len() >= 2, "helper calls must be recorded as references");
    assert!(refs.iter().any(|r| r.kind == ReferenceKind::Call));
}
