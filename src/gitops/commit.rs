use super::shell;
use super::GitError;
use crate::gitops::Repo;

impl Repo {
    pub fn commit(&self, message: &str) -> Result<(), GitError> {
        shell::commit(&self.path, message)
    }

    pub fn commit_amend(&self, message: &str) -> Result<(), GitError> {
        shell::commit_amend(&self.path, message)
    }

    pub fn cherry_pick(&self, oid: &str) -> Result<(), GitError> {
        shell::cherry_pick(&self.path, oid)
    }

    pub fn cherry_pick_abort(&self) -> Result<(), GitError> {
        shell::cherry_pick_abort(&self.path)
    }

    pub fn cherry_pick_continue(&self) -> Result<(), GitError> {
        shell::cherry_pick_continue(&self.path)
    }
}
