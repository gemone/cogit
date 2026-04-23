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
        // Use git log -g to get stash entries with refs in one command (avoids O(n) rev-parse calls)
        let output = self.git_cmd(&[
            "log",
            "-g",
            "--format=%H %gd %s",
            "--all",
            "refs/stash",
        ])?;
        let mut entries = Vec::new();

        for line in output.lines() {
            if line.is_empty() {
                continue;
            }
            // Format: <hash> stash@{0} <message>
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() < 3 {
                continue;
            }
            let hash = parts[0].to_string();
            let refname = parts[1].to_string();
            let message = parts[2..].join(" ");

            // Extract index from "stash@{N}"
            let index = if let Some(start) = refname.find('{') {
                if let Some(end) = refname.find('}') {
                    refname[start + 1..end].parse::<usize>().unwrap_or(0)
                } else {
                    0
                }
            } else {
                0
            };

            // Parse message: "shelve:<name>:<timestamp>" or "shelve:<name>:<timestamp>:staged"
            if let Some(shelve_prefix) = message.find("shelve:") {
                let after_shelve = &message[shelve_prefix + 7..];
                let parts: Vec<&str> = after_shelve.splitn(3, ':').collect();
                if parts.len() >= 2 {
                    let name = parts[0].to_string();
                    let date = parts.get(1).unwrap_or(&"").to_string();
                    let has_staged = parts.len() >= 3 && parts[2].contains("staged");

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

        let mut args = vec!["stash", "push", "-m", &message];
        if !include_staged {
            // Only keep index when NOT including staged changes (i.e., when include_staged=false,
            // we want to preserve the index state as-is after stashing)
            args.push("--keep-index");
        }
        let _output = self.git_cmd(&args)?;
        Ok(format!("Created shelve: {}", name))
    }

    pub fn shelve_apply(&self, index: usize, pop: bool) -> Result<String> {
        let refname = format!("stash@{{{}}}", index);
        let cmd = if pop { "pop" } else { "apply" };
        let output = self.git_cmd(&["stash", cmd, "--index", &refname])?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_repo(dir_name: &str) -> (Repository, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("cogit-test-{}", dir_name));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let repo = Repository::open(&dir).unwrap();
        repo.git_cmd(&["init"]).unwrap();
        repo.git_cmd(&["config", "user.name", "Test"]).unwrap();
        repo.git_cmd(&["config", "user.email", "test@test.com"]).unwrap();
        fs::write(dir.join("file.txt"), "initial\n").unwrap();
        repo.git_cmd(&["add", "."]).unwrap();
        repo.git_cmd(&["commit", "-m", "initial"]).unwrap();
        (repo, dir)
    }

    #[test]
    fn test_shelve_create_and_list() {
        let (repo, dir) = setup_test_repo("shelve");
        fs::write(dir.join("file.txt"), "modified\n").unwrap();
        repo.shelve_create("my-shelve", false).unwrap();
        let shelves = repo.list_shelves().unwrap();
        assert_eq!(shelves.len(), 1);
        assert_eq!(shelves[0].name, "my-shelve");
    }

    #[test]
    fn test_shelve_apply() {
        let (repo, dir) = setup_test_repo("shelve-apply");
        fs::write(dir.join("file.txt"), "modified\n").unwrap();
        repo.shelve_create("test-apply", false).unwrap();

        // apply (pop=false) should restore but keep stash
        repo.shelve_apply(0, false).unwrap();
        let content = fs::read_to_string(dir.join("file.txt")).unwrap();
        assert_eq!(content, "modified\n");
        let shelves = repo.list_shelves().unwrap();
        assert_eq!(shelves.len(), 1); // stash preserved
    }

    #[test]
    fn test_shelve_pop() {
        let (repo, dir) = setup_test_repo("shelve-pop");
        fs::write(dir.join("file.txt"), "modified\n").unwrap();
        repo.shelve_create("test-pop", false).unwrap();
        let shelves = repo.list_shelves().unwrap();
        assert_eq!(shelves.len(), 1);

        repo.shelve_apply(0, true).unwrap(); // pop=true
        let content = fs::read_to_string(dir.join("file.txt")).unwrap();
        assert_eq!(content, "modified\n");
        let shelves = repo.list_shelves().unwrap();
        assert!(shelves.is_empty());
    }

    #[test]
    fn test_shelve_drop() {
        let (repo, dir) = setup_test_repo("shelve-drop");
        fs::write(dir.join("file.txt"), "modified\n").unwrap();
        repo.shelve_create("test-drop", false).unwrap();
        let shelves = repo.list_shelves().unwrap();
        assert_eq!(shelves.len(), 1);

        repo.shelve_drop(0).unwrap();
        let shelves = repo.list_shelves().unwrap();
        assert!(shelves.is_empty());
    }

    #[test]
    fn test_shelve_apply_by_name() {
        let (repo, dir) = setup_test_repo("shelve-by-name");
        fs::write(dir.join("file.txt"), "modified\n").unwrap();
        repo.shelve_create("named-shelve", false).unwrap();

        repo.shelve_apply_by_name("named-shelve", false).unwrap();
        let content = fs::read_to_string(dir.join("file.txt")).unwrap();
        assert_eq!(content, "modified\n");
    }
}
