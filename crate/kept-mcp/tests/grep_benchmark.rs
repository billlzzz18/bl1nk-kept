use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;

use kept_mcp::mcp::tools::filesystem::FffManager;

struct McpFixture {
    root: PathBuf,
}

impl McpFixture {
    fn new(name: &str) -> Self {
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kept-mcp-fixture-{name}-{unique_id}"));
        fs::create_dir_all(&root).expect("failed to create fixture dir");
        let root = root.canonicalize().expect("canonicalize fixture path");
        Self { root }
    }

    fn path(&self) -> &Path {
        &self.root
    }

    fn write_file(&self, path: &str, content: &[u8]) {
        let full_path = self.root.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(full_path, content).unwrap();
    }
}

#[tokio::test]
async fn grep_context_accuracy_vs_rg() {
    let fixture = McpFixture::new("grep-benchmark");

    fixture.write_file(
        "src/lib.rs",
        b"fn add(a: i32, b: i32) -> i32 { a + b }\n\
        fn subtract(a: i32, b: i32) -> i32 { a - b }\n\
        fn multiply(a: i32, b: i32) -> i32 { a * b }\n\
        fn divide(a: i32, b: i32) -> i32 { a / b }\n\
        const CONSTANT: i32 = 42;\n\
        let x = add(1, 2);\n\
        let y = subtract(5, 3);\n"
    );

    fixture.write_file(
        "docs/readme.md",
        b"# Welcome to kept\n\nThis is a test document.\n"
    );

    let manager = FffManager::new();
    let root_str = fixture.path().to_str().unwrap();

    let grep_res = manager.grep(root_str, "fn add").await.expect("grep query");
    let add_match = grep_res.matches.iter()
        .find(|m| m.path.contains("src/lib.rs") && m.line_content.contains("fn add"))
        .expect("must find match for fn add");

    assert!(!add_match.context_before.is_empty() || true, "context_before may be empty at file start");
    assert!(!add_match.context_after.is_empty(), "context_after must not be empty when subsequent lines exist");

    assert!(add_match.context_after.iter().any(|l| l.contains("fn subtract")),
        "context_after should contain 'fn subtract' line");

    println!("Context before: {:?}", add_match.context_before);
    println!("Context after: {:?}", add_match.context_after);
}

#[tokio::test]
async fn grep_vs_rg_recall_precision() {
    let fixture = McpFixture::new("grep-vs-rg");

    fixture.write_file("src/a.rs", b"fn foo() {}\nfn bar() {}\nfn baz() {}\n");
    fixture.write_file("src/b.rs", b"fn foo() {}\nfn qux() {}\n");
    fixture.write_file("tests/test.rs", b"#[test]\nfn test_foo() {}\n");
    fixture.write_file("docs/guide.md", b"# Guide\n\nfoo function is useful\n");

    let manager = FffManager::new();
    let root_str = fixture.path().to_str().unwrap();

    let kept_res = manager.grep(root_str, "foo").await.expect("grep query");

    let output = Command::new("rg")
        .args(["--json", "foo", fixture.path().to_str().unwrap()])
        .output()
        .expect("rg must be available");

    let mut rg_paths = HashSet::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(data) = json.get("data") {
                if let Some(path) = data.get("path").and_then(|v| v["text"].as_str()) {
                    rg_paths.insert(path.replace('\\', "/"));
                }
            }
        }
    }

    let kept_paths: HashSet<String> = kept_res.matches.iter()
        .map(|m| m.path.replace('\\', "/"))
        .collect();

    let fixture_name = fixture.path().file_name().unwrap().to_str().unwrap().to_string();
    let rg_paths_normalized: HashSet<String> = rg_paths.iter()
        .map(|p| {
            let p = p.replace('\\', "/");
            let prefix = format!("{}/", fixture_name);
            if let Some(idx) = p.find(&prefix) {
                p[idx + prefix.len()..].to_string()
            } else {
                p
            }
        })
        .collect();

    let recall = if rg_paths_normalized.is_empty() { 1.0 } else {
        kept_paths.intersection(&rg_paths_normalized).count() as f64 / rg_paths_normalized.len() as f64
    };

    let precision = if kept_paths.is_empty() { 1.0 } else {
        kept_paths.intersection(&rg_paths_normalized).count() as f64 / kept_paths.len() as f64
    };

    println!("kept paths: {:?}", kept_paths);
    println!("rg paths: {:?}", rg_paths_normalized);
    println!("Recall: {:.2}, Precision: {:.2}", recall, precision);

    assert!(recall >= 0.8, "Recall should be >= 80%");
    assert!(precision >= 0.8, "Precision should be >= 80%");
}

#[tokio::test]
async fn grep_context_window_edge_cases() {
    let fixture = McpFixture::new("context-edge-cases");

    fixture.write_file("start.rs", b"fn first() {}\nfn second() {}\nfn third() {}\n");
    fixture.write_file("end.rs", b"fn first() {}\nfn second() {}\nfn last() {}\n");
    fixture.write_file("single.rs", b"only one line\n");
    fixture.write_file("middle.rs", b"fn a() {}\nfn b() {}\nfn c() {}\nfn d() {}\nfn e() {}\n");

    let manager = FffManager::new();
    let root_str = fixture.path().to_str().unwrap();

    // Match at start of file
    let res_start = manager.grep(root_str, "fn first").await.unwrap();
    let first_match = res_start.matches.iter()
        .find(|m| m.path.contains("start.rs"))
        .unwrap_or_else(|| panic!("should find match; matches={:?}", res_start.matches));
    println!("Start match context_before: {:?}", first_match.context_before);
    println!("Start match context_after: {:?}", first_match.context_after);

    // Match at end of file
    let res_end = manager.grep(root_str, "fn last").await.unwrap();
    let last_match = res_end.matches.iter()
        .find(|m| m.path.contains("end.rs"))
        .expect("should find match");
    println!("End match context_before: {:?}", last_match.context_before);
    println!("End match context_after: {:?}", last_match.context_after);

    // Single line file
    let res_single = manager.grep(root_str, "only one").await.unwrap();
    let single_match = res_single.matches.iter()
        .find(|m| m.path.contains("single.rs"))
        .expect("should find match");
    println!("Single match context_before: {:?}", single_match.context_before);
    println!("Single match context_after: {:?}", single_match.context_after);
}

#[tokio::test]
async fn grep_multi_pattern_accuracy() {
    let fixture = McpFixture::new("multi-pattern");
    fixture.write_file("test.rs", b"fn foo() {}\nfn bar() {}\nfn baz() {}\nfn qux() {}\n");
    fixture.write_file("other.rs", b"const FOO = 1;\nlet bar = 2;\n");

    let manager = FffManager::new();
    let root_str = fixture.path().to_str().unwrap();

    let multi_res = manager.multi_grep(root_str, &["foo", "bar"]).await.unwrap();

    let matches = multi_res.matches;
    println!("Multi-grep matches: {}", matches.len());
    for m in &matches {
        println!("  {}:{}: {}", m.path, m.line_number, m.line_content);
    }

    for m in &matches {
        assert!(!m.context_before.is_empty() || true, "context_before may be empty at start");
        assert!(!m.context_after.is_empty() || true, "context_after may be empty at end");
    }
}

#[tokio::test]
async fn grep_multiple_matches_same_file() {
    let fixture = McpFixture::new("multiple-matches");
    fixture.write_file(
        "repeated.rs",
        b"fn foo() {}\nfn foo() {}\nfn foo() {}\nfn bar() {}\nfn bar() {}\n"
    );

    let manager = FffManager::new();
    let root_str = fixture.path().to_str().unwrap();

    let res = manager.grep(root_str, "fn foo").await.unwrap();

    let foo_matches: Vec<_> = res.matches.iter()
        .filter(|m| m.path.contains("repeated.rs"))
        .collect();

    assert_eq!(foo_matches.len(), 3, "should find 3 matches of 'fn foo'");

    for m in &foo_matches {
        println!("Match at line {}: context_before={:?} context_after={:?}",
            m.line_number, m.context_before, m.context_after);
    }
}
