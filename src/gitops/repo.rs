use std::path::{Path, PathBuf};

use super::GitError;

pub struct Repo {
    pub(crate) inner: gix::Repository,
    pub(crate) path: PathBuf,
}

impl Repo {
    pub fn open(path: &Path) -> Result<Self, GitError> {
        let inner = gix::discover(path)?;
        let workdir = inner
            .work_dir()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| path.to_path_buf());
        Ok(Self {
            inner,
            path: workdir,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn head_shorthand(&self) -> Option<String> {
        self.inner
            .head_name()
            .ok()
            .flatten()
            .map(|r| r.shorten().to_string())
    }
}
