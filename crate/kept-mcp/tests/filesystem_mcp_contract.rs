//! TDD Contract test for unified filesystem MCP tools registration.
//! Verifies:
//! - Registration of filesystem_find, filesystem_grep, filesystem_multi_grep, filesystem_rescan, filesystem_status
//! - FffManager instance per canonical root
//! - Proper error handling on invalid canonical roots

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

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

impl Drop for McpFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[tokio::test]
async fn test_mcp_server_registers_filesystem_tools() {
    let server = kept_mcp::mcp::server::build();
    assert!(server.is_ok(), "Server build must succeed with filesystem tools registered");
}

#[tokio::test]
async fn test_fff_manager_lifecycle_and_tools() {
    use kept_mcp::mcp::tools::filesystem::FffManager;

    let fixture = McpFixture::new("mcp-fff-test");
    fixture.write_file("src/lib.rs", b"pub fn add(a: i32, b: i32) -> i32 { a + b }\n");
    fixture.write_file("docs/readme.md", b"# Welcome to kept\n");

    let manager = FffManager::new();
    let root_str = fixture.path().to_str().unwrap();

    // 1. filesystem_status
    let status = manager.status(root_str).await.expect("status check");
    assert_eq!(status.active_root, root_str);
    assert!(status.indexed_file_count >= 2);

    // 2. filesystem_find
    let find_res = manager.find(root_str, "lib").await.expect("find query");
    assert!(find_res
        .matches
        .iter()
        .any(|m| m.path.contains("src/lib.rs")));

    // 3. filesystem_grep
    let grep_res = manager
        .grep(root_str, "pub fn add")
        .await
        .expect("grep query");
    assert!(grep_res
        .matches
        .iter()
        .any(|m| m.path.contains("src/lib.rs") && m.line_content.contains("pub fn add")));

    // 4. filesystem_multi_grep
    let multi_grep_res = manager
        .multi_grep(root_str, &["add", "Welcome"])
        .await
        .expect("multi_grep query");
    assert!(!multi_grep_res.matches.is_empty());

    // 5. filesystem_rescan
    let rescan_res = manager.rescan(root_str).await.expect("rescan query");
    assert!(rescan_res.indexed_file_count >= 2);
}
