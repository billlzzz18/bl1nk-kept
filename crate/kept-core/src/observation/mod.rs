pub mod identity;
pub mod target;
pub mod types;

pub use identity::{ContentIdentity, Revision};
pub use target::{Target, TargetParseError};
pub use types::{Evidence, Observation, Provenance, Source};
