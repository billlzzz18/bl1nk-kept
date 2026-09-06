//! FFF scanner contract tests (TDD RED)
//! Verifies:
//! - Fixture with Git repo, .gitignore, .ignore, hidden files, node_modules, venv, .venv, __pycache__, target, symlink, binary file, modified file, and untracked file
//! - FFF ignore semantics and deterministic relative path output
//! - Metadata: path, name, extension, size, modified time, binary state, and Git status

use kept_core::scanner::{scan_directory, ScanOptions};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

struct TestFixture {
    root: PathBuf,
}

impl TestFixture {
    fn new(name: &str) -> Self {
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kept-fff-fixture-{name}-{unique_id}"));
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

    fn create_symlink(&self, target_relative: &str, link_relative: &str) {
        let target = self.root.join(target_relative);
        let link = self.root.join(link_relative);
        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent).expect("failed to create parent dir for symlink");
        }
        #[cfg(windows)]
        {
            if target.is_dir() {
                std::os::windows::fs::symlink_dir(&target, &link).ok();
            } else {
                std::os::windows::fs::symlink_file(&target, &link).ok();
            }
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target, &link).ok();
        }
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn fff_scanner_contract_ignores_and_metadata() {
    let fixture = TestFixture::new("comprehensive");
    let root = fixture.path();

    // 1. Initialize git repo in fixture (isolated from any active git hook env)
    let git_init = Command::new("git")
        .args(["init"])
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .current_dir(root)
        .output();
    let is_git_available = git_init.is_ok() && git_init.as_ref().unwrap().status.success();

    if is_git_available {
        Command::new("git")
            .args(["config", "user.name", "Test Runner"])
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .current_dir(root)
            .output()
            .ok();
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .current_dir(root)
            .output()
            .ok();
    }

    // 2. Setup ignore files
    // .gitignore
    fixture.write_file(".gitignore", b"ignored_by_gitignore.txt\nbuild/\n");
    // .ignore (ripgrep/FFF style)
    fixture.write_file(".ignore", b"ignored_by_dot_ignore.txt\n");

    // 3. Regular files (tracked / untracked / modified)
    fixture.write_file("src/main.rs", b"fn main() {\n    println!(\"hello\");\n}\n");
    fixture.write_file("docs/readme.md", b"# Documentation\n");
    fixture.write_file("data/binary.bin", &[0, 159, 255, 0, 12, 0, 4, 18]); // Binary file (null bytes)
    fixture.write_file("data/text.txt", b"plain text content\n");

    // 4. Files matching ignore rules
    fixture.write_file("ignored_by_gitignore.txt", b"should be ignored\n");
    fixture.write_file("build/artifact.o", b"build artifact\n");
    fixture.write_file("ignored_by_dot_ignore.txt", b"should be ignored\n");

    // 5. Common excluded directories (node_modules, venv, .venv, __pycache__, target)
    fixture.write_file("node_modules/package/index.js", b"console.log('pkg');");
    fixture.write_file("venv/bin/python", b"dummy python");
    fixture.write_file(".venv/pyvenv.cfg", b"home = /usr/bin");
    fixture.write_file("src/__pycache__/cache.pyc", b"dummy pyc");
    fixture.write_file("target/debug/app.exe", b"dummy binary");

    // 6. Hidden files
    fixture.write_file(".hidden_config", b"hidden file content");
    fixture.write_file(".config/settings.json", b"{\"key\":\"val\"}");

    // 7. Symlink
    fixture.create_symlink("docs/readme.md", "docs/link_readme.md");

    // 8. Commit some files to test Git status (tracked, modified, untracked)
    if is_git_available {
        Command::new("git")
            .args(["add", "src/main.rs", "docs/readme.md"])
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .current_dir(root)
            .output()
            .ok();
        Command::new("git")
            .args(["commit", "-m", "initial commit"])
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .current_dir(root)
            .output()
            .ok();

        // Modify tracked file
        fixture.write_file(
            "src/main.rs",
            b"fn main() {\n    println!(\"hello modified\");\n}\n",
        );

        // Untracked file is data/text.txt & data/binary.bin
    }

    // Run scanner with default ScanOptions (include_hidden = false)
    let options = ScanOptions::default();
    let index = scan_directory(root, &options).expect("scan_directory must succeed");

    // Collect all scanned relative paths normalized with forward slashes
    let paths: Vec<String> = index
        .files
        .iter()
        .map(|f| f.path.replace('\\', "/"))
        .collect();

    // Verify deterministic sorting (relative paths must be sorted ascending)
    let mut sorted_paths = paths.clone();
    sorted_paths.sort();
    assert_eq!(
        paths, sorted_paths,
        "File paths returned by scan must be deterministically sorted"
    );

    // Verify FFF Ignore Semantics:
    // MUST NOT contain ignored directories / files
    assert!(
        !paths.iter().any(|p| p.contains("node_modules")),
        "node_modules should be ignored by FFF semantics"
    );
    assert!(
        !paths.iter().any(|p| p.contains("venv")),
        "venv should be ignored by FFF semantics"
    );
    assert!(
        !paths.iter().any(|p| p.contains(".venv")),
        ".venv should be ignored by FFF semantics"
    );
    assert!(
        !paths.iter().any(|p| p.contains("__pycache__")),
        "__pycache__ should be ignored by FFF semantics"
    );
    assert!(
        !paths.iter().any(|p| p.contains("target/")),
        "target directory should be ignored by FFF semantics"
    );
    assert!(
        !paths.iter().any(|p| p.contains("ignored_by_gitignore.txt")),
        ".gitignore rules must be respected"
    );
    assert!(
        !paths.iter().any(|p| p.contains("build/")),
        ".gitignore directory rules must be respected"
    );
    assert!(
        !paths
            .iter()
            .any(|p| p.contains("ignored_by_dot_ignore.txt")),
        ".ignore rules must be respected"
    );
    assert!(
        !paths.iter().any(|p| p.starts_with(".hidden_config")),
        "hidden files must be ignored when include_hidden is false"
    );
    assert!(
        !paths.iter().any(|p| p.contains(".git/")),
        ".git internal directory must never be scanned"
    );

    // MUST contain standard allowed files
    assert!(
        paths.contains(&"src/main.rs".to_string()),
        "src/main.rs must be included"
    );
    assert!(
        paths.contains(&"docs/readme.md".to_string()),
        "docs/readme.md must be included"
    );
    assert!(
        paths.contains(&"data/binary.bin".to_string()),
        "data/binary.bin must be included"
    );
    assert!(
        paths.contains(&"data/text.txt".to_string()),
        "data/text.txt must be included"
    );

    // Verify metadata requirements: path, name, extension, size, modified time, is_binary, git_status
    let main_record = index
        .files
        .iter()
        .find(|f| f.path.replace('\\', "/") == "src/main.rs")
        .expect("src/main.rs record found");

    assert_eq!(main_record.name, "main.rs");
    assert_eq!(main_record.extension, "rs");
    assert!(main_record.size > 0);
    assert!(main_record.modified_unix > 0);

    // Test extended fields: is_binary and git_status
    // Serialized JSON contract check for FileRecord
    let json_val = serde_json::to_value(main_record).expect("must serialize to json");
    assert!(
        json_val.get("isBinary").is_some(),
        "FileRecord must include 'isBinary' field"
    );
    assert!(
        json_val.get("gitStatus").is_some(),
        "FileRecord must include 'gitStatus' field"
    );

    // Check binary detection
    let binary_record = index
        .files
        .iter()
        .find(|f| f.path.replace('\\', "/") == "data/binary.bin")
        .expect("data/binary.bin record found");
    let bin_json = serde_json::to_value(binary_record).expect("must serialize binary record");
    assert_eq!(
        bin_json.get("isBinary").and_then(|v| v.as_bool()),
        Some(true),
        "data/binary.bin must be detected as binary"
    );

    let text_record = index
        .files
        .iter()
        .find(|f| f.path.replace('\\', "/") == "data/text.txt")
        .expect("data/text.txt record found");
    let text_json = serde_json::to_value(text_record).expect("must serialize text record");
    assert_eq!(
        text_json.get("isBinary").and_then(|v| v.as_bool()),
        Some(false),
        "data/text.txt must not be binary"
    );

    // Check Git status if Git was available
    if is_git_available {
        // main.rs was modified
        assert_eq!(
            json_val.get("gitStatus").and_then(|v| v.as_str()),
            Some("modified"),
            "modified file must report gitStatus 'modified'"
        );

        // text.txt was untracked
        assert_eq!(
            text_json.get("gitStatus").and_then(|v| v.as_str()),
            Some("untracked"),
            "untracked file must report gitStatus 'untracked'"
        );
    }
}
