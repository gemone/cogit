use super::{GitError, Repo};

impl Repo {
    pub fn commit(&self, message: &str) -> Result<(), GitError> {
        let sig = self.inner.signature()?;
        let mut index = self.inner.index()?;
        let tree_id = index.write_tree()?;
        let tree = self.inner.find_tree(tree_id)?;

        let parent_commit = self.inner.head().ok().and_then(|h: git2::Reference<'_>| h.target()).and_then(|oid| self.inner.find_commit(oid).ok());

        let parents: Vec<&git2::Commit> = match &parent_commit {
            Some(c) => vec![c],
            None => vec![],
        };

        self.inner
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)?;

        Ok(())
    }

    pub fn commit_amend(&self, message: &str) -> Result<(), GitError> {
        let head = self.inner.head()?;
        let oid = head.target().ok_or_else(|| GitError::Other("no HEAD".to_string()))?;
        let commit = self.inner.find_commit(oid)?;
        let sig = self.inner.signature()?;
        let tree = commit.tree()?;

        commit.amend(Some("HEAD"), Some(&sig), Some(&sig), None, Some(message), Some(&tree))?;
        Ok(())
    }

    pub fn cherry_pick(&self, oid: &str) -> Result<(), GitError> {
        let oid = git2::Oid::from_str(oid).map_err(|e| GitError::Other(e.to_string()))?;
        let commit = self.inner.find_commit(oid)?;
        self.inner.cherrypick(&commit, None)?;
        Ok(())
    }

    pub fn cherry_pick_abort(&self) -> Result<(), GitError> {
        let head = self.inner.head()?;
        let oid = head.target().ok_or_else(|| GitError::Other("no HEAD".to_string()))?;
        let commit = self.inner.find_commit(oid)?;
        self.inner.reset(commit.as_object(), git2::ResetType::Hard, None)?;
        Ok(())
    }

    pub fn cherry_pick_continue(&self) -> Result<(), GitError> {
        let sig = self.inner.signature()?;
        let mut index = self.inner.index()?;
        let tree_id = index.write_tree()?;
        let tree = self.inner.find_tree(tree_id)?;
        let head = self.inner.head()?;
        let parent_oid = head.target().ok_or_else(|| GitError::Other("no HEAD".to_string()))?;
        let parent = self.inner.find_commit(parent_oid)?;

        self.inner
            .commit(Some("HEAD"), &sig, &sig, "cherry-pick continued", &tree, &[&parent])?;
        Ok(())
    }
}
