pub mod shell;
pub mod shelve;
pub mod stash;
pub mod types;

use anyhow::Result;
use std::path::Path;

pub struct Repository {
    path: std::path::PathBuf,
    gix_repo: Option<gix::Repository>,
}

impl Repository {
    pub fn open(path: &Path) -> Result<Self> {
        let gix_repo = gix::discover(path).ok();
        let repo = Self {
            path: path.to_path_buf(),
            gix_repo,
        };
        Ok(repo)
    }

    #[allow(dead_code)]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[allow(dead_code)]
    pub fn gix(&self) -> Option<&gix::Repository> {
        self.gix_repo.as_ref()
    }
}
