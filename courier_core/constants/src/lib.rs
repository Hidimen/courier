//! Provide constants.
//!
//! This lib only exports two symbols: [`COMMIT_ID`] and [`BRANCH`], acquired from git repository.
//! The value may change when new commits and branches are created.

/// Refers to current commit id.
pub const COMMIT_ID: &str = env!("COMMIT_ID");
/// Refers to current branch name.
pub const BRANCH: &str = env!("BRANCH");

/// Stores ipc socket name.
pub const IPC_NAME: &str = "courier.ipc";
