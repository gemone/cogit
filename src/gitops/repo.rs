use std::path::{Path, PathBuf};
use super::GitError;

pub struct Repo {
    pub(crate) inner: git2::Repository,
    pub(crate) path: PathBuf,
}

impl Repo {
    pub fn open(path: &Path) -> Result<Self, GitError> {
        let inner = git2::Repository::open(path)?;
        let path = inner.workdir().map(Path::to_path_buf).unwrap_or_else(|| path.to_path_buf());
        Ok(Self { inner, path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn head_shorthand(&self) -> Option<String> {
        self.inner.head().ok()?.shorthand().map(String::from)
    }
}
