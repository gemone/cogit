use crate::gitops::shell;
use crate::gitops::{GitError, Repo};

impl Repo {
    pub fn stage_path(&mut self, path: &str) -> Result<(), GitError> {
        shell::stage_path(&self.path, path)
    }

    pub fn unstage_path(&mut self, path: &str) -> Result<(), GitError> {
        shell::unstage_path(&self.path, path)
    }

    pub fn stage_all(&mut self) -> Result<(), GitError> {
        shell::stage_all(&self.path)
    }

    pub fn unstage_all(&mut self) -> Result<(), GitError> {
        shell::unstage_all(&self.path)
    }

    pub fn discard_path(&mut self, path: &str) -> Result<(), GitError> {
        shell::discard_path(&self.path, path)
    }
}
