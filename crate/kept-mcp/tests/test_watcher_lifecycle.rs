use kept_mcp::mcp::tools::watcher::RootWatcher;
use std::time::Duration;

#[tokio::test]
async fn root_watcher_emits_event_on_file_change() -> anyhow::Result<()> {
    let dir = std::env::temp_dir().join(format!("watcher-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("a.txt"), b"hello")?;

    let (tx, mut rx) = tokio::sync::mpsc::channel(64);
    let mut watcher = RootWatcher::new(dir.clone(), tx);
    watcher.start()?;

    // Create a file to trigger an event
    std::fs::write(dir.join("b.txt"), b"world")?;

    // Wait for debounce (500ms) + buffer
    let event = tokio::time::timeout(Duration::from_secs(3), rx.recv()).await;
    assert!(event.is_ok(), "watcher should emit an event within 3s");
    let evt = event?;
    assert!(evt.is_some(), "event should not be None");
    if let Some(change) = evt {
        assert_eq!(change.root, dir);
    }

    watcher.stop().await;
    let _ = std::fs::remove_dir_all(dir);
    Ok(())
}

#[tokio::test]
async fn root_watcher_starts_and_stops_without_panic() -> anyhow::Result<()> {
    let dir = std::env::temp_dir().join(format!("watcher-stop-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;

    let (tx, _rx) = tokio::sync::mpsc::channel(64);
    let mut watcher = RootWatcher::new(dir.clone(), tx);
    watcher.start()?;
    watcher.stop().await;

    let _ = std::fs::remove_dir_all(dir);
    Ok(())
}

#[tokio::test]
async fn ffmanager_auto_refreshes_index_after_file_change() {
    use kept_mcp::mcp::tools::filesystem::FffManager;

    let dir = std::env::temp_dir().join(format!("ffm-auto-{}", std::process::id()));
    if let Err(e) = std::fs::create_dir_all(&dir) {
        panic!("Failed to create temp dir: {e}");
    }
    if let Err(e) = std::fs::write(dir.join("a.txt"), b"hello") {
        panic!("Failed to write initial file: {e}");
    }

    let manager = FffManager::new();
    let status = manager
        .status(dir.to_str().unwrap_or_default())
        .await
        .expect("status call should succeed");
    assert_eq!(status.indexed_file_count, 1);

    manager
        .start_watcher(dir.to_str().unwrap_or_default())
        .await
        .expect("start_watcher should succeed");

    if let Err(e) = std::fs::write(dir.join("b.txt"), b"world") {
        panic!("Failed to write second file: {e}");
    }
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let status = manager
        .status(dir.to_str().unwrap_or_default())
        .await
        .expect("status call should succeed after change");
    assert!(
        status.indexed_file_count >= 2,
        "index should auto-refresh to >= 2 files, got {}",
        status.indexed_file_count
    );

    manager.stop_all_watchers().await;
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn rescan_applies_incremental_delta() {
    use kept_mcp::mcp::tools::filesystem::FffManager;

    let dir = std::env::temp_dir().join(format!("rescan-delta-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.txt"), b"hello").unwrap();

    let manager = FffManager::new();
    let status1 = manager.status(dir.to_str().unwrap()).await.unwrap();
    assert_eq!(status1.indexed_file_count, 1);

    std::fs::write(dir.join("b.txt"), b"world").unwrap();
    let status2 = manager.rescan(dir.to_str().unwrap()).await.unwrap();
    assert_eq!(status2.indexed_file_count, 2);

    std::fs::remove_file(dir.join("b.txt")).unwrap();
    let status3 = manager.rescan(dir.to_str().unwrap()).await.unwrap();
    assert_eq!(status3.indexed_file_count, 1);

    manager.stop_all_watchers().await;
    let _ = std::fs::remove_dir_all(dir);
}
