use std::path::{Path, PathBuf};

use super::shell;
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

    pub fn diff_for_file(&self, path: &str) -> Result<String, GitError> {
        shell::diff_for_file(&self.path, path)
    }

    pub fn diff_staged_for_file(&self, path: &str) -> Result<String, GitError> {
        shell::diff_staged_for_file(&self.path, path)
    }

    pub fn list_tags(&self) -> Result<Vec<String>, GitError> {
        let output = shell::list_tags(&self.path)?;
        Ok(output
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect())
    }

    pub fn log_oneline(&self, n: usize) -> Result<String, GitError> {
        shell::log_oneline(&self.path, n)
    }
}
