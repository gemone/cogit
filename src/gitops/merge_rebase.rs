use super::{GitError, Repo};

pub fn is_rebasing(repo: &Repo) -> bool {
    repo.inner.state() == git2::RepositoryState::Rebase
        || repo.inner.state() == git2::RepositoryState::RebaseInteractive
        || repo.inner.state() == git2::RepositoryState::RebaseMerge
}

pub fn is_merging(repo: &Repo) -> bool {
    repo.inner.state() == git2::RepositoryState::Merge
}

impl Repo {
    pub fn merge(&self, branch: &str) -> Result<bool, GitError> {
        let obj = self.inner.revparse_single(branch)?;
        let annotated = self.inner.find_annotated_commit(obj.id())?;
        self.inner.merge(&[&annotated], None, None)?;

        let has_conflicts = self.inner.index()?.has_conflicts();
        Ok(!has_conflicts)
    }

    pub fn rebase(&self, branch: &str) -> Result<(), GitError> {
        let branch_obj = self.inner.revparse_single(branch)?;
        let branch_commit = self.inner.find_annotated_commit(branch_obj.id())?;
        let head_obj = self.inner.revparse_single("HEAD")?;
        let head_commit = self.inner.find_annotated_commit(head_obj.id())?;

        let mut rebase = self.inner.rebase(
            Some(&head_commit),
            Some(&branch_commit),
            None,
            None,
        )?;

        while let Some(op) = rebase.next() {
            let _ = op?;
        }

        let sig = self.inner.signature()?;
        rebase.commit(None, &sig, None)?;
        rebase.finish(None)?;
        Ok(())
    }

    pub fn rebase_continue(&self) -> Result<(), GitError> {
        let mut rebase = self.inner.open_rebase(None)?;
        let sig = self.inner.signature()?;
        rebase.commit(None, &sig, None)?;
        rebase.finish(None)?;
        Ok(())
    }

    pub fn rebase_abort(&self) -> Result<(), GitError> {
        let mut rebase = self.inner.open_rebase(None)?;
        rebase.abort()?;
        Ok(())
    }

    pub fn rebase_skip(&self) -> Result<(), GitError> {
        let mut rebase = self.inner.open_rebase(None)?;
        rebase.commit(None, &self.inner.signature()?, None)?;
        rebase.finish(None)?;
        Ok(())
    }

    pub fn pull(&self, remote_name: &str, branch: &str) -> Result<(), GitError> {
        let mut remote = self.inner.find_remote(remote_name)?;
        remote.fetch(&[branch], None, None)?;
        let fetch_head = self.inner.find_reference("FETCH_HEAD")?;
        let fetch_commit = fetch_head.peel_to_commit()?;
        let head = self.inner.head()?.peel_to_commit()?;

        let mut index = self.inner.merge_commits(&head, &fetch_commit, None)?;
        if index.has_conflicts() {
            return Err(GitError::MergeConflict);
        }

        let tree_id = index.write_tree_to(&self.inner)?;
        let tree = self.inner.find_tree(tree_id)?;
        let sig = self.inner.signature()?;
        self.inner.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &format!("Merge branch '{}' of {}", branch, remote_name),
            &tree,
            &[&head, &fetch_commit],
        )?;

        Ok(())
    }
}
