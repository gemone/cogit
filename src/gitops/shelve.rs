use super::Repository;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct ShelveEntry {
    pub name: String,
    pub date: String,
}

impl Repository {
    pub fn list_shelves(&self) -> Result<Vec<ShelveEntry>> {
        // Git doesn't have native "shelves" - we implement via refs
        let output = self
            .git_cmd(&[
                "for-each-ref",
                "--format=%(refname:short) %(creatordate:short)",
                "refs/shelves/",
            ])
            .unwrap_or_default();
        let mut entries = Vec::new();
        for line in output.lines() {
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            let name = parts[0].trim_start_matches("shelves/").to_string();
            let date = parts.get(1).unwrap_or(&"").to_string();
            entries.push(ShelveEntry { name, date });
        }
        Ok(entries)
    }

    pub fn shelve_apply(&self, name: &str) -> Result<String> {
        let refname = format!("shelves/{}", name);
        let output = self.git_cmd(&["cherry-pick", &refname])?;
        Ok(output)
    }

    pub fn shelve_drop(&self, name: &str) -> Result<String> {
        let refname = format!("refs/shelves/{}", name);
        let output = self.git_cmd(&["update-ref", "-d", &refname])?;
        Ok(output)
    }

    pub fn shelve_create(&self, name: &str) -> Result<String> {
        let refname = format!("refs/shelves/{}", name);
        let output = self.git_cmd(&["stash", "create"])?;
        let hash = output.trim();
        if hash.is_empty() {
            anyhow::bail!("Nothing to shelve");
        }
        self.git_cmd(&["update-ref", &refname, hash])?;
        Ok(format!("Created shelve: {}", name))
    }
}
