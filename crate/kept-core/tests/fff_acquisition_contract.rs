use kept_core::scanner::fff::{FffAcquisitionMode, FffScanner};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

struct TestFixture {
    root: PathBuf,
}

impl TestFixture {
    fn new(name: &str) -> Self {
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kept-fff-look-view-{name}-{unique_id}"));
        fs::create_dir_all(&root).expect("failed to create fixture dir");
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

impl Drop for TestFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn test_fff_look_and_view_acquisition() {
    let fixture = TestFixture::new("look-view-contract");
    fixture.write_file("src/main.rs", b"fn main() {\n    println!(\"hello\");\n}\n");
    fixture.write_file("data/binary.bin", &[0x00, 0xFF, 0xFE, 0x12, 0x34]);

    let mut scanner = FffScanner::new(fixture.path()).expect("initialize scanner");

    // 1. Look mode: acquire metadata without loading full content
    let look_obs = scanner
        .acquire("src/main.rs", FffAcquisitionMode::Look)
        .expect("acquire look");

    assert_eq!(look_obs.source.target.to_string(), "file://src/main.rs");
    assert_eq!(look_obs.source.adapter, "fff");
    assert!(look_obs.content.is_none()); // Look mode never loads body content
    assert_eq!(look_obs.metadata["is_binary"], false);
    assert_eq!(look_obs.metadata["size"], 37);

    // 2. View mode: acquire metadata and materialize content
    let view_obs = scanner
        .acquire("src/main.rs", FffAcquisitionMode::View)
        .expect("acquire view");

    assert_eq!(view_obs.source.target.to_string(), "file://src/main.rs");
    assert!(view_obs.content.is_some());
    assert_eq!(view_obs.content.as_deref(), Some("fn main() {\n    println!(\"hello\");\n}\n"));

    // 3. Binary file view: view on binary should retain is_binary=true and omit utf-8 content or encode safely
    let bin_obs = scanner
        .acquire("data/binary.bin", FffAcquisitionMode::View)
        .expect("acquire bin");
    assert_eq!(bin_obs.metadata["is_binary"], true);
    assert!(bin_obs.content.is_none());
}
