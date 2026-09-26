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
pub mod terminal;

pub use agent::{AgentSubcommand, handle_agent};
pub use config::{
    ConfigCommands, DefaultCommands, ProfileCommands, ScopeCommands, ScopeExceptionCommands,
    ScopeSettingCommands, config_field_default_value, handle_config, set_profile_field,
    unset_profile_field,
};
pub use doc::{DocCommands, handle_doc};
pub use doctor::{
    DoctorFinding, DoctorReport, DoctorRunResult, DoctorStatus, InvalidConfigRepair,
    doctor_editor_at, handle_doctor, repair_invalid_config_at, repair_missing_config_at,
};
pub use duplicates::{
    DuplicateAction, choose_duplicate_action, duplicate_action_from_selection,
    handle_task_duplicates, run_duplicate_action,
};
pub use fs::{DuplicateCommands, FsCommands, handle_duplicate_scan, handle_fs};
pub use group::{GroupCommands, GroupFieldCommands, handle_group};
pub use registry::{RegistryCommands, handle_registry};
pub use scan::{
    FindRequest, ScanReviewAction, TaskScanResult, build_scan_space_summary,
    create_or_refresh_scan, handle_task_find, handle_task_review, handle_task_scan,
    naming_review_summary, naming_review_summary_at, scan_review_action_from_selection,
    scan_review_summary,
};
pub use setup::{
    SetupAction, SetupResult, handle_setup, setup_action_from_selection, setup_user_config_at,
};
pub use terminal::{TerminalCommands, handle_terminal};
