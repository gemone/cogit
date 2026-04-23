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
    fn test_stash_create_and_list() {
        let (repo, _dir) = setup_test_repo("stash");
        fs::write(_dir.join("file.txt"), "modified\n").unwrap();
        repo.stash_create(Some("test-stash")).unwrap();
        let list = repo.stash_list().unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].message.contains("test-stash"));
    }

    #[test]
    fn test_stash_pop() {
        let (repo, dir) = setup_test_repo("stash-pop");
        fs::write(dir.join("file.txt"), "modified\n").unwrap();
        repo.stash_create(Some("test-pop")).unwrap();
        let list = repo.stash_list().unwrap();
        assert_eq!(list.len(), 1);

        repo.stash_pop(0).unwrap();
        let content = fs::read_to_string(dir.join("file.txt")).unwrap();
        assert_eq!(content, "modified\n");
        let list = repo.stash_list().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn test_stash_apply() {
        let (repo, dir) = setup_test_repo("stash-apply");
        fs::write(dir.join("file.txt"), "modified\n").unwrap();
        repo.stash_create(Some("test-apply")).unwrap();

        repo.stash_apply(0).unwrap();
        let content = fs::read_to_string(dir.join("file.txt")).unwrap();
        assert_eq!(content, "modified\n");
        let list = repo.stash_list().unwrap();
        assert_eq!(list.len(), 1); // stash preserved after apply
    }

    #[test]
    fn test_stash_drop() {
        let (repo, dir) = setup_test_repo("stash-drop");
        fs::write(dir.join("file.txt"), "modified\n").unwrap();
        repo.stash_create(Some("test-drop")).unwrap();
        let list = repo.stash_list().unwrap();
        assert_eq!(list.len(), 1);

        repo.stash_drop(0).unwrap();
        let list = repo.stash_list().unwrap();
        assert!(list.is_empty());
    }
}
