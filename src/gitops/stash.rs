use super::{GitError, Repo, StashEntry};

impl Repo {
    pub fn stash_save(&mut self, msg: &str, include_untracked: bool) -> Result<(), GitError> {
        let sig = self.inner.signature()?;
        let mut opts = git2::StashFlags::DEFAULT;
        if include_untracked {
            opts |= git2::StashFlags::INCLUDE_UNTRACKED;
        }
        self.inner.stash_save2(&sig, Some(msg), Some(opts))?;
        Ok(())
    }

    pub fn stash_pop(&mut self, index: usize) -> Result<(), GitError> {
        self.inner.stash_pop(index, None)?;
        Ok(())
    }

    pub fn stash_apply(&mut self, index: usize) -> Result<(), GitError> {
        self.inner.stash_apply(index, None)?;
        Ok(())
    }

    pub fn stash_drop(&mut self, index: usize) -> Result<(), GitError> {
        self.inner.stash_drop(index)?;
        Ok(())
    }

    pub fn stash_list(&mut self) -> Result<Vec<StashEntry>, GitError> {
        let mut entries = Vec::new();
        self.inner.stash_foreach(|i, msg: &str, _oid: &git2::Oid| {
            entries.push(StashEntry {
                index: i,
                message: msg.to_string(),
            });
            true
        })?;
        Ok(entries)
    }
}
