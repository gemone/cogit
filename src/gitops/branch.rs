use super::{BranchInfo, GitError, Repo};

impl Repo {
    pub fn branches(&self) -> Result<Vec<BranchInfo>, GitError> {
        let mut branches = Vec::new();
        let branch_iter = self.inner.branches(None)?;

        for branch_tuple in branch_iter {
            let (branch, branch_type): (git2::Branch<'_>, git2::BranchType) = branch_tuple?;
            let name = branch.name()?.unwrap_or("???").to_string();
            let is_remote = branch_type == git2::BranchType::Remote;

            let upstream = if !is_remote {
                branch.upstream().ok().and_then(|u: git2::Branch<'_>| u.name().ok().flatten().map(String::from))
            } else {
                None
            };

            branches.push(BranchInfo {
                name,
                is_remote,
                upstream,
            });
        }

        Ok(branches)
    }

    pub fn checkout(&self, name: &str) -> Result<(), GitError> {
        let obj = self.inner.revparse_single(name)?;
        self.inner.checkout_tree(&obj, None)?;
        self.inner.set_head_detached(obj.id())?;
        Ok(())
    }

    pub fn create_branch(&self, name: &str, base: &str) -> Result<(), GitError> {
        let obj = self.inner.revparse_single(base)?;
        let commit = obj.peel_to_commit()?;
        self.inner.branch(name, &commit, false)?;
        Ok(())
    }

    pub fn delete_branch(&self, name: &str, _force: bool) -> Result<(), GitError> {
        let mut branch = self
            .inner
            .find_branch(name, git2::BranchType::Local)
            .map_err(|_| GitError::BranchNotFound(name.to_string()))?;
        branch.delete()?;
        Ok(())
    }

    pub fn set_upstream(&self, local: &str, remote: &str) -> Result<(), GitError> {
        let mut branch = self
            .inner
            .find_branch(local, git2::BranchType::Local)
            .map_err(|_| GitError::BranchNotFound(local.to_string()))?;
        branch.set_upstream(Some(remote))?;
        Ok(())
    }
}
