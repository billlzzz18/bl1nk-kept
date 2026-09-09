//! Phase 1.1 (multi-language): Python, JS/TS, Go symbol extraction contract tests

use kept_core::source_graph::{DefinitionKind, IndexBuilder};
use std::path::Path;

#[test]
fn python_def_and_class_captured() {
    let code = r#"
import os

def compute_total(items):
    return sum(items)

class PriceCalculator:
    def apply_discount(self, amount):
        return amount * 0.9

# def commented_out() ต้องไม่ถูกจับ
"#;
    let index = IndexBuilder::parse_file(Path::new("calc.py"), code);
    assert_eq!(index.find_definitions("compute_total").len(), 1);
    assert_eq!(index.find_definitions("PriceCalculator").len(), 1);
    assert_eq!(index.find_definitions("apply_discount").len(), 1);
    let class_def = &index.find_definitions("PriceCalculator")[0];
    assert_eq!(class_def.kind, DefinitionKind::Struct); // class → Struct
    assert_eq!(index.imports.len(), 1);
    assert_eq!(index.imports[0].path, "os");
}

#[test]
fn javascript_function_and_class_captured() {
    let code = r#"
const config = require('config');

function renderTemplate(name) {
  return `hello ${name}`;
}

class WidgetHandler {
  mount() {}
}

// function ghosted() ใน comment
"#;
    let index = IndexBuilder::parse_file(Path::new("widget.js"), code);
    assert_eq!(index.find_definitions("renderTemplate").len(), 1);
    assert_eq!(index.find_definitions("WidgetHandler").len(), 1);
    assert_eq!(index.find_definitions("mount").len(), 1);
    // NOTE-013: require() เป็น call expression ไม่ใช่ import declaration — ไม่นับเป็น import
    assert!(index.imports.is_empty());
}

#[test]
fn typescript_interface_and_function_captured() {
    let code = r#"
import type { Settings } from './settings';

export interface UserAccount {
  id: string;
}

export function buildAccount(id: string): UserAccount {
  return { id };
}
"#;
    let index = IndexBuilder::parse_file(Path::new("account.ts"), code);
    assert_eq!(index.find_definitions("UserAccount").len(), 1);
    assert_eq!(index.find_definitions("buildAccount").len(), 1);
    assert_eq!(index.imports.len(), 1);
}

#[test]
fn go_func_struct_interface_captured() {
    let code = r#"
package main

import "fmt"

type Store struct {
    Name string
}

type Keeper interface {
    Keep() error
}

func (s *Store) Keep() error {
    fmt.Println(s.Name)
    return nil
}

func NewStore(name string) *Store {
    return &Store{Name: name}
}
"#;
    let index = IndexBuilder::parse_file(Path::new("store.go"), code);
    assert_eq!(index.find_definitions("Store").len(), 1);
    assert_eq!(index.find_definitions("Keeper").len(), 1);
    assert_eq!(index.find_definitions("NewStore").len(), 1);
    assert_eq!(index.imports.len(), 1);
    assert_eq!(index.imports[0].path, "fmt");
}

#[test]
fn unsupported_extension_returns_empty_index() {
    let index = IndexBuilder::parse_file(Path::new("notes.txt"), "fn not_rust() {}");
    assert!(index.definitions.is_empty());
    assert!(index.indexed_files.is_empty());
}

#[test]
fn dogfood_parse_real_workspace_rust_file() {
    // Dogfooding: parse ไฟล์จริงใน workspace ต้องได้ definitions ที่รู้จัก
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let syntax_path = manifest.join("src/source_graph/syntax.rs");
    let content = std::fs::read_to_string(&syntax_path)
        .unwrap_or_else(|e| panic!("must read real workspace file {syntax_path:?}: {e}"));
    let index = IndexBuilder::parse_file(&syntax_path, &content);
    assert!(
        index.find_definitions("parse_rust_ast").len() >= 1,
        "parse_rust_ast must be extracted from real syntax.rs"
    );
    assert!(
        index.find_definitions("parse_rust_outline").len() >= 1,
        "parse_rust_outline must be extracted from real syntax.rs"
    );
}

#[test]
fn dogfood_parse_real_workspace_python_tool() {
    // Dogfooding: parse tools/extract_obsidian_corpus.py ของ repo จริง
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("repo root must be two levels above kept-core");
    let tool_path = repo_root.join("tools/extract_obsidian_corpus.py");
    if !tool_path.exists() {
        return; // ไม่มี fixture ใน CI ก็ไม่ fail
    }
    let content = std::fs::read_to_string(&tool_path).unwrap_or_default();
    let index = IndexBuilder::parse_file(&tool_path, &content);
    assert!(
        !index.definitions.is_empty(),
        "python tool must yield definitions"
    );
}
