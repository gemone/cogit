use std::path::{Path, PathBuf};

use super::shell;
use super::{CommitInfo, GitError};

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

    pub fn current_branch_name(&self) -> String {
        self.head_shorthand().unwrap_or_else(|| "HEAD".to_string())
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

    pub fn is_dirty(&self) -> bool {
        shell::is_dirty(&self.path)
    }

    pub fn fetch_remote(&self, remote: &str) -> Result<(), GitError> {
        shell::fetch(&self.path, remote)?;
        Ok(())
    }

    pub fn fetch_all_remotes(&self) -> Result<(), GitError> {
        shell::fetch_all(&self.path)?;
        Ok(())
    }

    pub fn pull_merge(&self, remote: &str, branch: &str) -> Result<(), GitError> {
        shell::pull_merge(&self.path, remote, branch)
    }

    pub fn pull_rebase(&self, remote: &str, branch: &str) -> Result<(), GitError> {
        shell::pull_rebase(&self.path, remote, branch)
    }

    pub fn rename_branch(&self, old: &str, new: &str) -> Result<(), GitError> {
        shell::rename_branch(&self.path, old, new)
    }

    pub fn log_detailed(&self, count: usize) -> Result<Vec<CommitInfo>, GitError> {
        let output = shell::log_detailed(&self.path, count)?;
        let mut commits = Vec::new();
        let mut lines = output.lines();
        loop {
            let oid = match lines.next() {
                Some(l) if !l.is_empty() => l.to_string(),
                _ => break,
            };
            let hash = lines.next().unwrap_or("").to_string();
            let author = lines.next().unwrap_or("").to_string();
            let date = lines.next().unwrap_or("").to_string();
            let subject = lines.next().unwrap_or("").to_string();
            let _end = lines.next(); // consume ---END---
            commits.push(CommitInfo {
                hash,
                oid,
                author,
                date,
                subject,
            });
        }
        Ok(commits)
    }

    pub fn show_commit(&self, oid: &str) -> Result<String, GitError> {
        shell::show_commit(&self.path, oid)
    }

    pub fn diff_commit(&self, oid: &str) -> Result<String, GitError> {
        shell::diff_commit(&self.path, oid)
    }

    pub fn push_current(&self) -> Result<(), GitError> {
        let branch = self.head_shorthand().unwrap_or_default();
        if branch.is_empty() {
            return Err(GitError::Other("No current branch".to_string()));
        }
        // Push to origin by default; set upstream if needed
        let mut cmd = shell::git_cmd(&self.path);
        cmd.args(["push", "-u", "origin", &branch]);
        shell::run_git(&mut cmd)?;
        Ok(())
    }
}
