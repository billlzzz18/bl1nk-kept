//! Multi-language tree-sitter symbol extraction (Python, JS/TS, Go, HTML, Markdown)
//! NOTE-P1.2: แทนที่ node-walk แบบ imperative ด้วย tree-sitter Query API + .scm files
//! tree-sitter 0.25 ใช้ StreamingIterator ไม่ใช่ Iterator มาตรฐานสำหรับ QueryMatches

use super::{DefinitionKind, ImportRecord, ScopeGraphIndex, SymbolDefinition};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Language as TsLanguage, Parser, Query, QueryCursor};

/// ภาษาที่ parser รองรับ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Html,
    Markdown,
}

/// เลือกภาษาจากนามสกุลไฟล์
pub fn language_from_extension(extension: &str) -> Option<Language> {
    match extension {
        "rs" => Some(Language::Rust),
        "py" => Some(Language::Python),
        "js" | "jsx" | "mjs" | "cjs" => Some(Language::JavaScript),
        "ts" | "tsx" | "mts" | "cts" => Some(Language::TypeScript),
        "go" => Some(Language::Go),
        "html" | "htm" => Some(Language::Html),
        "md" | "mdx" => Some(Language::Markdown),
        _ => None,
    }
}

/// tree-sitter grammar language สำหรับแต่ละ Language variant
fn ts_language(language: Language) -> TsLanguage {
    match language {
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        Language::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::Go => tree_sitter_go::LANGUAGE.into(),
        Language::Html => tree_sitter_html::LANGUAGE.into(),
        Language::Markdown => tree_sitter_md_025::LANGUAGE.into(),
    }
}

/// query source (.scm) สำหรับแต่ละภาษา — embed ณ compile time
fn query_source(language: Language) -> &'static str {
    match language {
        Language::Rust => include_str!("../parsers/rust.scm"),
        Language::Python => include_str!("../parsers/python.scm"),
        Language::JavaScript => include_str!("../parsers/javascript.scm"),
        Language::TypeScript => include_str!("../parsers/typescript.scm"),
        Language::Go => include_str!("../parsers/go.scm"),
        Language::Html => include_str!("../parsers/html.scm"),
        Language::Markdown => include_str!("../parsers/markdown.scm"),
    }
}

/// สร้าง parser สำหรับภาษาที่กำหนด
fn parser_for(language: Language) -> Option<Parser> {
    let mut parser = Parser::new();
    let lang = ts_language(language);
    if parser.set_language(&lang).is_err() {
        return None;
    }
    Some(parser)
}

/// แปลง capture name → DefinitionKind
fn capture_to_kind(capture_name: &str) -> Option<DefinitionKind> {
    match capture_name {
        "definition.function" => Some(DefinitionKind::Function),
        "definition.class" => Some(DefinitionKind::Struct),
        "definition.struct" => Some(DefinitionKind::Struct),
        "definition.enum" => Some(DefinitionKind::Enum),
        "definition.interface" => Some(DefinitionKind::Trait),
        "definition.trait" => Some(DefinitionKind::Trait),
        "definition.type_alias" | "definition.type" => Some(DefinitionKind::TypeAlias),
        "definition.macro" => Some(DefinitionKind::Other("macro".to_string())),
        "definition.heading1"
        | "definition.heading2"
        | "definition.heading3"
        | "definition.element"
        | "definition.id"
        | "definition.code_block"
        | "definition.script" => Some(DefinitionKind::Other(capture_name.to_string())),
        _ => None,
    }
}

/// Parse source code ตามภาษาด้วย Query API แล้ว extract symbols ลง index
pub fn parse_source(language: Language, path_str: &str, content: &str) -> Option<ScopeGraphIndex> {
    let mut parser = parser_for(language)?;
    let ts_lang = ts_language(language);
    let tree = parser.parse(content, None)?;
    let source = query_source(language);

    // NOTE-001: Query::new อาจ fail ถ้า .scm มี syntax error — log แล้ว return None
    let query = match Query::new(&ts_lang, source) {
        Ok(q) => q,
        Err(e) => {
            eprintln!("Query::new failed for {:?}: {:?}", language, e);
            return None;
        }
    };

    let mut index = ScopeGraphIndex::new();
    index.indexed_files.insert(path_str.to_string());

    let mut cursor = QueryCursor::new();
    let source_bytes = content.as_bytes();

    // NOTE-022: tree-sitter 0.25 ใช้ StreamingIterator — ต้องเรียก .advance() + .get()
    let mut matches = cursor.matches(&query, tree.root_node(), source_bytes);
    while let Some(mat) = {
        matches.advance();
        matches.get()
    } {
        for capture in mat.captures {
            let capture_name = query.capture_names()[capture.index as usize];
            // ข้าม internal captures (_*)
            if capture_name.starts_with('_') {
                continue;
            }
            let node = capture.node;
            let text = std::str::from_utf8(&source_bytes[node.start_byte()..node.end_byte()])
                .unwrap_or("")
                .trim()
                .trim_matches(|c| c == '"' || c == '\'')
                .to_string();

            if text.is_empty() {
                continue;
            }

            if capture_name == "import.statement" || capture_name.starts_with("import.") {
                // Parse the entire use_declaration node text
                // Format: "use path::to::module;" or "use path::to::module as alias;" or "use path::to::*;"
                let raw = text
                    .strip_prefix("use ")
                    .unwrap_or(&text)
                    .trim_end_matches(';')
                    .trim();
                let is_wildcard = raw.ends_with("::*");
                let (path_part, alias) = match raw.find(" as ") {
                    Some(idx) => {
                        (raw[..idx].trim().to_string(), Some(raw[idx + 4..].trim().to_string()))
                    }
                    None => (raw.to_string(), None),
                };
                index.imports.push(ImportRecord {
                    path: path_part,
                    alias,
                    file_path: path_str.to_string(),
                    line: node.start_position().row + 1,
                    is_wildcard,
                });
                continue;
            }

            if let Some(kind) = capture_to_kind(capture_name) {
                let position = node.start_position();
                index.definitions.push(SymbolDefinition {
                    name: text,
                    kind,
                    file_path: path_str.to_string(),
                    line: position.row + 1,
                    column: position.column + 1,
                    doc: None,
                });
            }
        }
    }

    Some(index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_def_and_class_captured() {
        let code = "def greet(name):\n    pass\n\nclass MyClass:\n    pass\n";
        let index = parse_source(Language::Python, "test.py", code).expect("python parse");
        let names: Vec<&str> = index.definitions.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"greet"), "function must be captured");
        assert!(names.contains(&"MyClass"), "class must be captured");
    }

    #[test]
    fn javascript_function_and_class_captured() {
        let code = "function hello() {}\nclass World {}\n";
        let index = parse_source(Language::JavaScript, "test.js", code).expect("js parse");
        let names: Vec<&str> = index.definitions.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"hello"));
        assert!(names.contains(&"World"));
    }

    #[test]
    fn typescript_interface_and_function_captured() {
        let code = "interface Foo {}\nfunction bar(): void {}\n";
        let index = parse_source(Language::TypeScript, "test.ts", code).expect("ts parse");
        let names: Vec<&str> = index.definitions.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"Foo"));
        assert!(names.contains(&"bar"));
    }

    #[test]
    fn go_func_struct_interface_captured() {
        let code =
            "package main\nfunc Run() {}\ntype Server struct {}\ntype Handler interface {}\n";
        let index = parse_source(Language::Go, "test.go", code).expect("go parse");
        let names: Vec<&str> = index.definitions.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"Run"));
        assert!(names.contains(&"Server"));
        assert!(names.contains(&"Handler"));
    }

    #[test]
    fn unsupported_extension_returns_none() {
        assert!(language_from_extension("txt").is_none());
        assert!(language_from_extension("csv").is_none());
    }

    #[test]
    fn dogfood_parse_real_workspace_rust_file() {
        use crate::source_graph::syntax;
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/source_graph/multilang.rs");
        let content = std::fs::read_to_string(path).expect("multilang.rs must be readable");
        let index = parse_source(Language::Rust, path, &content);
        // NOTE: parse_source may return None if parser/query fails — log for debug
        let index = match index {
            Some(idx) => idx,
            None => {
                // Debug: try parsing with syntax::parse_rust_ast to compare
                let syntax_index = syntax::parse_rust_ast(path, &content);
                panic!(
                    "rust dogfood parse returned None. syntax::parse_rust_ast returned: {:?}",
                    syntax_index.is_some()
                );
            }
        };
        let fn_names: Vec<&str> = index
            .definitions
            .iter()
            .filter(|d| matches!(d.kind, DefinitionKind::Function))
            .map(|d| d.name.as_str())
            .collect();
        assert!(
            fn_names.contains(&"parse_source"),
            "parse_source must be in definitions, got: {:?}",
            fn_names
        );
    }
}
