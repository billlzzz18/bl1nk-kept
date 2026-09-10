//! CLI domain command handlers and subcommand routers.

pub mod agent;
pub mod config;
pub mod doc;
pub mod doctor;
pub mod duplicates;
pub mod fs;
pub mod group;
pub mod registry;
pub mod scan;
pub mod setup;

pub use agent::{handle_agent, AgentSubcommand};
pub use config::{
    config_field_default_value, handle_config, set_profile_field, unset_profile_field,
    ConfigCommands, DefaultCommands, ProfileCommands, ScopeCommands, ScopeExceptionCommands,
    ScopeSettingCommands,
};
pub use doc::{handle_doc, DocCommands};
pub use doctor::{
    doctor_editor_at, handle_doctor, repair_invalid_config_at, repair_missing_config_at,
    DoctorFinding, DoctorReport, DoctorRunResult, DoctorStatus, InvalidConfigRepair,
};
pub use duplicates::{
    choose_duplicate_action, duplicate_action_from_selection, handle_task_duplicates,
    run_duplicate_action, DuplicateAction,
};
pub use fs::{handle_duplicate_scan, handle_fs, DuplicateCommands, FsCommands};
pub use group::{handle_group, GroupCommands, GroupFieldCommands};
pub use registry::{handle_registry, RegistryCommands};
pub use scan::{
    build_scan_space_summary, create_or_refresh_scan, handle_task_find, handle_task_review,
    handle_task_scan, naming_review_summary, naming_review_summary_at,
    scan_review_action_from_selection, scan_review_summary, FindRequest, ScanReviewAction,
    TaskScanResult,
};
pub use setup::{
    handle_setup, setup_action_from_selection, setup_user_config_at, SetupAction, SetupResult,
};
