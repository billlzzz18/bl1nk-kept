//! File watcher for auto-rescan — watches a root and emits debounced change events.

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc as std_mpsc;
use std::time::Duration;
use tokio::sync::mpsc;

// NOTE-001: debounce interval 500ms ป้องกัน burst events จาก editor save (หลาย write ต่อเนื่อง)
const DEBOUNCE_MS: u64 = 500;

/// Event emitted by [`RootWatcher`] after debounce.
#[derive(Debug, Clone)]
pub struct RootChangeEvent {
    pub root: PathBuf,
}

/// Watches a single workspace root and emits debounced change events.
pub struct RootWatcher {
    root: PathBuf,
    tx: mpsc::Sender<RootChangeEvent>,
    _watcher: Option<RecommendedWatcher>,
}

impl RootWatcher {
    pub fn new(root: PathBuf, tx: mpsc::Sender<RootChangeEvent>) -> Self {
        Self {
            root,
            tx,
            _watcher: None,
        }
    }

    /// Start watching. Spawns a blocking notify thread and a debounce task.
    pub fn start(&mut self) -> anyhow::Result<()> {
        let (notify_tx, notify_rx) = std_mpsc::channel::<notify::Result<notify::Event>>();
        let mut watcher = RecommendedWatcher::new(
            notify_tx,
            notify::Config::default().with_poll_interval(Duration::from_secs(1)),
        )?;
        watcher.watch(&self.root, RecursiveMode::Recursive)?;

        let root = self.root.clone();
        let tx = self.tx.clone();

        // NOTE-002: blocking thread อ่าน notify_rx แล้วส่ง signal เมื่อมี event
        // ใช้ std::thread::spawn เพราะ notify_rx ไม่ Send สำหรับ spawn_blocking loop
        let (signal_tx, signal_rx) = mpsc::channel::<()>(16);
        std::thread::spawn(move || {
            while let Ok(Ok(ev)) = notify_rx.recv() {
                if matches!(
                    ev.kind,
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                ) && signal_tx.blocking_send(()).is_err()
                {
                    break;
                }
            }
        });

        // NOTE-003: debounce task — รับ signal แล้วส่ง event หลัง debounce window
        tokio::spawn(async move {
            let mut pending = false;
            let mut signal_rx = signal_rx;
            loop {
                tokio::select! {
                    sig = signal_rx.recv() => {
                        match sig {
                            Some(()) => { pending = true; }
                            None => break,
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(DEBOUNCE_MS)), if pending => {
                        pending = false;
                        if tx.send(RootChangeEvent { root: root.clone() }).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        self._watcher = Some(watcher);
        Ok(())
    }

    /// Stop watching by dropping the notify watcher.
    pub async fn stop(&mut self) {
        self._watcher.take();
    }
}
