use super::shell;
use super::{GitError, Repo};

pub fn is_rebasing(repo: &Repo) -> bool {
    shell::is_rebasing(&repo.path)
}

pub fn is_merging(repo: &Repo) -> bool {
    shell::is_merging(&repo.path)
}

impl Repo {
    pub fn merge(&self, branch: &str) -> Result<bool, GitError> {
        shell::merge(&self.path, branch)
    }

    pub fn rebase(&self, branch: &str) -> Result<(), GitError> {
        shell::rebase(&self.path, branch)
    }

    pub fn rebase_continue(&self) -> Result<(), GitError> {
        shell::rebase_continue(&self.path)
    }

    pub fn rebase_abort(&self) -> Result<(), GitError> {
        shell::rebase_abort(&self.path)
    }

    pub fn rebase_skip(&self) -> Result<(), GitError> {
        shell::rebase_skip(&self.path)
    }

    pub fn pull(&self, remote_name: &str, branch: &str) -> Result<(), GitError> {
        shell::pull(&self.path, remote_name, branch)
    }
}
