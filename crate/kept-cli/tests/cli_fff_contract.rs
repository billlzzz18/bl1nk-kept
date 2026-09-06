//! TDD contract tests for FFF-backed CLI scan and find operations.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

struct CliFixture {
    root: PathBuf,
}

impl CliFixture {
    fn new(name: &str) -> Self {
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kept-cli-fixture-{name}-{unique_id}"));
        fs::create_dir_all(&root).expect("failed to create fixture dir");
        let root = root.canonicalize().expect("canonicalize fixture path");
        Self { root }
    }

    fn path(&self) -> &Path {
        &self.root
    }

    fn write_file(&self, relative: &str, content: &[u8]) -> PathBuf {
        let full = self.root.join(relative);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).expect("failed to create parent dir");
        }
        let mut file = File::create(&full).expect("failed to create file");
        file.write_all(content).expect("failed to write content");
        full
    }
}

impl Drop for CliFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn test_cli_scan_and_find_fff_contract() {
    let fixture = CliFixture::new("cli-scan-find");
    let root = fixture.path();

    // 1. Setup sample files & git
    let git_init = Command::new("git")
        .args(["init"])
        .current_dir(root)
        .output();
    let is_git_available = git_init.is_ok() && git_init.as_ref().unwrap().status.success();

    if is_git_available {
        Command::new("git")
            .args(["config", "user.name", "Test Runner"])
            .current_dir(root)
            .output()
            .ok();
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(root)
            .output()
            .ok();
    }

    fixture.write_file("src/main.rs", b"fn main() {\n    println!(\"hello\");\n}\n");
    fixture.write_file("docs/readme.md", b"# Documentation\n");
    fixture.write_file("docs/api.md", b"# API Documentation\n");
    fixture.write_file(".gitignore", b"ignored.txt\n");
    fixture.write_file("ignored.txt", b"ignored file\n");

    if is_git_available {
        Command::new("git")
            .args(["add", ".gitignore", "src/main.rs", "docs/readme.md"])
            .current_dir(root)
            .output()
            .ok();
        Command::new("git")
            .args(["commit", "-m", "initial commit"])
            .current_dir(root)
            .output()
            .ok();
    }

    let index_path = root.join("scan_index.json");

    // 2. Build kept binary path
    let bin_path = env!("CARGO_BIN_EXE_kept");

    // Run `kept scan <root> --output <index_path> --json`
    let output = Command::new(bin_path)
        .args([
            "scan",
            root.to_str().unwrap(),
            "--output",
            index_path.to_str().unwrap(),
            "--json",
        ])
        .output()
        .expect("failed to execute kept scan");

    assert!(output.status.success(), "kept scan must succeed");
    let scan_stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        scan_stdout.contains("fileCount"),
        "scan output should contain fileCount"
    );
    assert!(
        scan_stdout.contains("totalSize"),
        "scan output should contain totalSize"
    );
    assert!(index_path.exists(), "Snapshot file must be created");

    // 3. Run `kept find <root> --index <index_path> --json`
    let output_find = Command::new(bin_path)
        .args([
            "find",
            root.to_str().unwrap(),
            "--index",
            index_path.to_str().unwrap(),
            "--json",
        ])
        .output()
        .expect("failed to execute kept find");

    assert!(output_find.status.success(), "kept find must succeed");
    let find_stdout = String::from_utf8_lossy(&output_find.stdout);
    assert!(
        find_stdout.contains("src/main.rs"),
        "find output should contain src/main.rs"
    );
    assert!(
        find_stdout.contains("docs/readme.md"),
        "find output should contain docs/readme.md"
    );
    assert!(
        !find_stdout.contains("ignored.txt"),
        "find output must not contain ignored.txt"
    );
}
