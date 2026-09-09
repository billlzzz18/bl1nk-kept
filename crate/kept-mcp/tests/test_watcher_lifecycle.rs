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
