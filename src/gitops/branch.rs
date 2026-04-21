use super::shell;
use super::{BranchInfo, GitError, Repo};

impl Repo {
    pub fn branches(&self) -> Result<Vec<BranchInfo>, GitError> {
        let mut branches = Vec::new();

        // List local branches
        let local_output = shell::run_git(
            &mut shell::git_cmd(&self.path).args(["branch", "--format=%(refname:short)"]),
        )?;
        for name in local_output.lines() {
            if name.is_empty() {
                continue;
            }
            let upstream = self.get_upstream(name);
            branches.push(BranchInfo {
                name: name.to_string(),
                is_remote: false,
                upstream,
            });
        }

        // List remote branches
        let remote_output = shell::run_git(
            &mut shell::git_cmd(&self.path)
                .args(["branch", "-r", "--format=%(refname:short)"]),
        )?;
        for name in remote_output.lines() {
            if name.is_empty() {
                continue;
            }
            branches.push(BranchInfo {
                name: name.to_string(),
                is_remote: true,
                upstream: None,
            });
        }

        Ok(branches)
    }

    fn get_upstream(&self, branch: &str) -> Option<String> {
        let output = shell::run_git(
            &mut shell::git_cmd(&self.path)
                .args(["config", &format!("branch.{}.remote", branch)]),
        )
        .ok();
        let remote = output?;
        let merge = shell::run_git(
            &mut shell::git_cmd(&self.path)
                .args(["config", &format!("branch.{}.merge", branch)]),
        )
        .ok()?;
        let merge_short = merge.strip_prefix("refs/heads/").unwrap_or(&merge);
        Some(format!("{}/{}", remote, merge_short))
    }

    pub fn checkout(&self, name: &str) -> Result<(), GitError> {
        shell::checkout(&self.path, name)
    }

    pub fn create_branch(&self, name: &str, base: &str) -> Result<(), GitError> {
        shell::create_branch(&self.path, name, base)
    }

    pub fn delete_branch(&self, name: &str, force: bool) -> Result<(), GitError> {
        shell::delete_branch(&self.path, name, force)
    }

    pub fn set_upstream(&self, local: &str, remote: &str) -> Result<(), GitError> {
        shell::set_upstream(&self.path, local, remote)
    }
}
