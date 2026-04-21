use super::shell;
use super::{GitError, Repo, StashEntry};

impl Repo {
    pub fn stash_save(&mut self, msg: &str, include_untracked: bool) -> Result<(), GitError> {
        shell::stash_save(&self.path, msg, include_untracked)
    }

    pub fn stash_pop(&mut self, index: usize) -> Result<(), GitError> {
        shell::stash_pop(&self.path, index)
    }

    pub fn stash_apply(&mut self, index: usize) -> Result<(), GitError> {
        shell::stash_apply(&self.path, index)
    }

    pub fn stash_drop(&mut self, index: usize) -> Result<(), GitError> {
        shell::stash_drop(&self.path, index)
    }

    pub fn stash_list(&mut self) -> Result<Vec<StashEntry>, GitError> {
        let output = shell::stash_list(&self.path)?;
        let mut entries = Vec::new();

        for line in output.lines() {
            // Format: stash@{0}: On branch: message
            if let Some(rest) = line.strip_prefix("stash@{") {
                if let Some(brace_end) = rest.find("}: ") {
                    if let Ok(idx) = rest[..brace_end].parse::<usize>() {
                        let colon_pos = rest.find("}: ").unwrap();
                        let msg = &rest[colon_pos + 3..];
                        entries.push(StashEntry {
                            index: idx,
                            message: msg.to_string(),
                        });
                    }
                }
            }
        }

        Ok(entries)
    }
}
