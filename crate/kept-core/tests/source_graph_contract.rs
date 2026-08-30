use kept_core::{load_index, save_index, FileEvent, IndexBuilder, IndexManager, ScopeGraphIndex};
use std::fs;

#[test]
fn test_scope_graph_index_contract_and_incremental_flow() {
    let temp_dir = std::env::temp_dir().join(format!("kept-graph-contract-{}", std::process::id()));
    fs::create_dir_all(&temp_dir).expect("temporary contract test dir created");

    let sub_dir = temp_dir.join("src");
    let hidden_dir = temp_dir.join(".git");
    fs::create_dir_all(&sub_dir).unwrap();
    fs::create_dir_all(&hidden_dir).unwrap();

    let file_a = sub_dir.join("service.rs");
    let file_b = sub_dir.join("handler.rs");
    let hidden_file = hidden_dir.join("config.rs");
    let binary_file = sub_dir.join("blob.rs");

    fs::write(
        &file_a,
        r#"
        use std::collections::HashMap as Map;
        use std::sync::*;

        pub trait UserService {
            fn get_user(&self) -> bool;
        }

        pub struct UserServiceImpl;

        impl UserService for UserServiceImpl {
            fn get_user(&self) -> bool {
                true
            }
        }
    "#,
    )
    .unwrap();

    fs::write(
        &file_b,
        r#"
        use crate::service::UserService;

        pub fn handle_request() {
            let s = UserServiceImpl;
        }
    "#,
    )
    .unwrap();

    fs::write(&hidden_file, "pub fn hidden_fn() {}").unwrap();
    fs::write(&binary_file, [0u8, 159, 255, 0, b'f', b'n']).unwrap();

    // 1. IndexBuilder recursive scan with limits and binary skip
    let builder = IndexBuilder::new()
        .skip_hidden(true)
        .max_file_size_bytes(1024 * 1024)
        .build_batch_size(10);

    let index = builder.build_from_directory(&temp_dir);

    assert_eq!(index.find_definitions("UserService").len(), 1);
    assert_eq!(index.find_definitions("UserServiceImpl").len(), 1);
    assert_eq!(index.find_definitions("hidden_fn").len(), 0);

    let stats = index.stats();
    assert!(stats.total_definitions >= 3);
    assert!(stats.total_imports >= 3);
    assert_eq!(stats.total_implementations, 1);

    // 2. Persistence round-trip
    let index_file = temp_dir.join("source_graph.json");
    save_index(&index, &index_file).expect("save_index success");
    let loaded: ScopeGraphIndex = load_index(&index_file).expect("load_index success");
    assert_eq!(loaded.stats(), stats);

    // 3. Incremental IndexManager & snapshot
    let manager = IndexManager::from_index(loaded);
    let snapshot = manager.get_snapshot();
    assert_eq!(snapshot.stats(), stats);

    // Incremental modification event
    fs::write(
        &file_b,
        r#"
        pub fn new_handler_fn() {}
    "#,
    )
    .unwrap();

    let events = vec![
        FileEvent::modified(file_b.clone()),
        FileEvent::modified(file_b.clone()),
    ];
    manager.apply_events(&events);

    let snap2 = manager.get_snapshot();
    assert_eq!(snap2.find_definitions("new_handler_fn").len(), 1);

    // Incremental deletion
    fs::remove_file(&file_b).ok();
    manager.apply_events(&[FileEvent::deleted(file_b.clone())]);
    let snap3 = manager.get_snapshot();
    assert_eq!(snap3.find_definitions("new_handler_fn").len(), 0);

    fs::remove_dir_all(&temp_dir).ok();
}
