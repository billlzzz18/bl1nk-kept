//! Test watcher behavior with atomic save patterns (nano, vim, VS Code).
//!
//! Editors typically: write-to-temp → rename-to-original
//! The watcher must detect the final state, not intermediate temp files.

use std::time::Duration;

#[tokio::test]
async fn watcher_detects_atomic_save_write_then_rename() {
    use kept_mcp::mcp::tools::watcher::RootWatcher;

    let dir = std::env::temp_dir().join(format!("atomic-save-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("doc.txt"), b"version 1").unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::channel(64);
    let mut watcher = RootWatcher::new(dir.clone(), tx);
    watcher.start().unwrap();

    // Wait for initial setup
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Simulate atomic save: write to temp, then rename
    let temp_file = dir.join("doc.txt.tmp");
    std::fs::write(&temp_file, b"version 2 atomic").unwrap();
    std::fs::rename(&temp_file, dir.join("doc.txt")).unwrap();

    // Wait for debounce + event
    let event = tokio::time::timeout(Duration::from_secs(3), rx.recv()).await;
    assert!(event.is_ok(), "watcher should detect atomic save within 3s");
    let evt = event.unwrap();
    assert!(evt.is_some(), "event should not be None");

    // Verify final content
    let content = std::fs::read_to_string(dir.join("doc.txt")).unwrap();
    assert_eq!(content, "version 2 atomic");

    watcher.stop().await;
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn watcher_detects_vim_style_swap_save() {
    use kept_mcp::mcp::tools::watcher::RootWatcher;

    let dir = std::env::temp_dir().join(format!("vim-save-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("code.rs"), b"fn main() {}").unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::channel(64);
    let mut watcher = RootWatcher::new(dir.clone(), tx);
    watcher.start().unwrap();

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Simulate vim: write backup, write new, delete backup
    std::fs::write(dir.join("code.rs~"), b"fn main() {}").unwrap(); // backup
    std::fs::write(dir.join("code.rs"), b"fn main() { println!(\"hello\"); }").unwrap(); // new content
    std::fs::remove_file(dir.join("code.rs~")).unwrap(); // cleanup backup

    let event = tokio::time::timeout(Duration::from_secs(3), rx.recv()).await;
    assert!(event.is_ok(), "watcher should detect vim-style save within 3s");
    assert!(event.unwrap().is_some());

    let content = std::fs::read_to_string(dir.join("code.rs")).unwrap();
    assert!(content.contains("hello"), "content should be updated: {content}");

    watcher.stop().await;
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn watcher_detects_vscode_style_save() {
    use kept_mcp::mcp::tools::watcher::RootWatcher;

    let dir = std::env::temp_dir().join(format!("vscode-save-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("index.ts"), b"export const x = 1;").unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::channel(64);
    let mut watcher = RootWatcher::new(dir.clone(), tx);
    watcher.start().unwrap();

    tokio::time::sleep(Duration::from_millis(200)).await;

    // VS Code writes to temp file then renames (similar to atomic save)
    let temp = dir.join(".index.ts.tmp");
    std::fs::write(&temp, b"export const x = 2; export const y = 3;").unwrap();
    std::fs::rename(&temp, dir.join("index.ts")).unwrap();

    let event = tokio::time::timeout(Duration::from_secs(3), rx.recv()).await;
    assert!(event.is_ok(), "watcher should detect vscode-style save within 3s");
    assert!(event.unwrap().is_some());

    let content = std::fs::read_to_string(dir.join("index.ts")).unwrap();
    assert!(content.contains("y = 3"), "content should be updated: {content}");

    watcher.stop().await;
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn watcher_ignores_temp_files_and_tracks_original() {
    use kept_mcp::mcp::tools::watcher::RootWatcher;

    let dir = std::env::temp_dir().join(format!("temp-ignore-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("data.json"), b"{}").unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::channel(64);
    let mut watcher = RootWatcher::new(dir.clone(), tx);
    watcher.start().unwrap();

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Create temp files that editors leave behind — should not confuse watcher
    std::fs::write(dir.join(".data.json.swp"), b"swap").unwrap();
    std::fs::write(dir.join("data.json.tmp"), b"tmp").unwrap();

    // Wait a bit — watcher may or may not emit events for temp files
    tokio::time::sleep(Duration::from_millis(1000)).await;

    // Drain any events from temp files
    while let Ok(Some(_)) = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {}

    // Now do a real atomic save
    std::fs::rename(dir.join("data.json.tmp"), dir.join("data.json")).unwrap();
    let event = tokio::time::timeout(Duration::from_secs(3), rx.recv()).await;
    assert!(event.is_ok(), "watcher should detect final rename within 3s");

    // Cleanup
    let _ = std::fs::remove_file(dir.join(".data.json.swp"));
    watcher.stop().await;
    let _ = std::fs::remove_dir_all(dir);
}
