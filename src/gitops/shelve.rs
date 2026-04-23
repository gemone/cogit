use super::Repository;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct ShelveEntry {
    pub index: usize,
    pub name: String,
    pub date: String,
    pub hash: String,
    pub has_staged: bool,
}

impl Repository {
    pub fn list_shelves(&self) -> Result<Vec<ShelveEntry>> {
        let output = self.git_cmd(&["stash", "list"])?;
        let mut entries = Vec::new();

        for line in output.lines() {
            if line.is_empty() {
                continue;
            }
            // Format: stash@{0}: On <branch>: shelve:<name>:<timestamp>
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

                // Parse message: " On <branch>: shelve:<name>:<timestamp>"
                let message = rest.trim().to_string();
                if let Some(shelve_prefix) = message.find("shelve:") {
                    let after_shelve = &message[shelve_prefix + 7..];
                    // Format: name:timestamp (with optional staged marker)
                    let parts: Vec<&str> = after_shelve.splitn(3, ':').collect();
                    if parts.len() >= 2 {
                        let name = parts[0].to_string();
                        let date = parts.get(1).unwrap_or(&"").to_string();
                        let has_staged = parts.len() >= 3 && parts[2].contains("staged");

                        let hash = self
                            .git_cmd(&["rev-parse", &format!("stash@{{{}}}", index)])
                            .unwrap_or_default()
                            .trim()
                            .to_string();

                        entries.push(ShelveEntry {
                            index,
                            name,
                            date,
                            hash,
                            has_staged,
                        });
                    }
                }
            }
        }
        Ok(entries)
    }

    pub fn shelve_create(&self, name: &str, include_staged: bool) -> Result<String> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .to_string();

        let message = if include_staged {
            format!("shelve:{}:{}:staged", name, timestamp)
        } else {
            format!("shelve:{}:{}", name, timestamp)
        };

        let args = vec!["stash", "push", "-m", &message];
        let _output = self.git_cmd(&args)?;
        Ok(format!("Created shelve: {}", name))
    }

    pub fn shelve_apply(&self, index: usize, pop: bool) -> Result<String> {
        let refname = format!("stash@{{{}}}", index);
        let cmd = if pop { "pop" } else { "apply" };
        let output = self.git_cmd(&["stash", cmd, &refname])?;
        Ok(output)
    }

    pub fn shelve_drop(&self, index: usize) -> Result<String> {
        let refname = format!("stash@{{{}}}", index);
        let output = self.git_cmd(&["stash", "drop", &refname])?;
        Ok(output)
    }

    pub fn shelve_show(&self, index: usize) -> Result<String> {
        let refname = format!("stash@{{{}}}", index);
        let output = self.git_cmd(&["stash", "show", "-p", &refname])?;
        Ok(output)
    }

    pub fn shelve_apply_by_name(&self, name: &str, pop: bool) -> Result<String> {
        let entries = self.list_shelves()?;
        let entry = entries
            .iter()
            .find(|e| e.name == name)
            .ok_or_else(|| anyhow::anyhow!("Shelve '{}' not found", name))?;
        self.shelve_apply(entry.index, pop)
    }

    pub fn shelve_drop_by_name(&self, name: &str) -> Result<String> {
        let entries = self.list_shelves()?;
        let entry = entries
            .iter()
            .find(|e| e.name == name)
            .ok_or_else(|| anyhow::anyhow!("Shelve '{}' not found", name))?;
        self.shelve_drop(entry.index)
    }
}
