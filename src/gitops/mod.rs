pub mod branch;
pub mod commit;
pub mod ignore;
pub mod merge_rebase;
pub mod remote;
pub mod repo;
pub mod shelve;
pub mod stash;
pub mod status;

use thiserror::Error;

pub use repo::Repo;
pub use status::{FileStatus, WorktreeFile};

#[derive(Debug, Error)]
pub enum GitError {
    #[error("git2 error: {0}")]
    Git2(#[from] git2::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("not a git repository")]
    NotARepo,
    #[error("invalid reference: {0}")]
    InvalidReference(String),
    #[error("merge conflict")]
    MergeConflict,
    #[error("rebase in progress")]
    RebaseInProgress,
    #[error("stash not found: index {0}")]
    StashNotFound(usize),
    #[error("shelve not found: {0}")]
    ShelveNotFound(String),
    #[error("branch not found: {0}")]
    BranchNotFound(String),
    #[error("other: {0}")]
    Other(String),
}

#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub is_remote: bool,
    pub upstream: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StashEntry {
    pub index: usize,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ShelveInfo {
    pub name: String,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RemoteInfo {
    pub name: String,
    pub url: Option<String>,
}
