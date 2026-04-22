use super::Repository;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct StashEntry {
    pub index: usize,
    pub hash: String,
    pub message: String,
}

impl Repository {
    pub fn stash_list(&self) -> Result<Vec<StashEntry>> {
        let output = self.git_cmd(&["stash", "list"])?;
        let mut entries = Vec::new();
        for line in output.lines() {
            if line.is_empty() {
                continue;
            }
            // Format: stash@{0}: On branch: message
            if let Some(colon_pos) = line.find(':') {
                let prefix = &line[..colon_pos];
                let rest = &line[colon_pos + 1..];
                // Extract index from "stash@{N}"
                let index = if let Some(start) = prefix.find('{') {
                    if let Some(end) = prefix.find('}') {
                        prefix[start + 1..end].parse::<usize>().unwrap_or(0)
                    } else {
                        0
                    }
                } else {
                    0
                };
                let message = rest.trim().to_string();
                let hash = self
                    .git_cmd(&["rev-parse", &format!("stash@{{{}}}", index)])
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                entries.push(StashEntry {
                    index,
                    hash,
                    message,
                });
            }
        }
        Ok(entries)
    }

    pub fn stash_create(&self, message: Option<&str>) -> Result<String> {
        let args = if let Some(msg) = message {
            vec!["stash", "push", "-m", msg]
        } else {
            vec!["stash", "push"]
        };
        let output = self.git_cmd(&args.to_vec())?;
        Ok(output)
    }

    pub fn stash_pop(&self, index: usize) -> Result<String> {
        let refname = format!("stash@{{{}}}", index);
        let output = self.git_cmd(&["stash", "pop", &refname])?;
        Ok(output)
    }

    pub fn stash_apply(&self, index: usize) -> Result<String> {
        let refname = format!("stash@{{{}}}", index);
        let output = self.git_cmd(&["stash", "apply", &refname])?;
        Ok(output)
    }

    pub fn stash_drop(&self, index: usize) -> Result<String> {
        let refname = format!("stash@{{{}}}", index);
        let output = self.git_cmd(&["stash", "drop", &refname])?;
        Ok(output)
    }
}
