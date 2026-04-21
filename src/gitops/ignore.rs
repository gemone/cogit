use super::{GitError, Repo};
use std::fs;
use std::path::Path;

impl Repo {
    pub fn ignore_patterns(&self, dir: &Path) -> Result<Vec<String>, GitError> {
        let gitignore = dir.join(".gitignore");
        if !gitignore.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&gitignore)?;
        Ok(content.lines().map(|s| s.to_string()).collect())
    }

    pub fn add_ignore(&self, dir: &Path, pattern: &str) -> Result<(), GitError> {
        let gitignore = dir.join(".gitignore");
        let mut content = if gitignore.exists() {
            fs::read_to_string(&gitignore)?
        } else {
            String::new()
        };
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(pattern);
        content.push('\n');
        fs::write(&gitignore, content)?;
        Ok(())
    }

    pub fn remove_ignore(&self, dir: &Path, pattern: &str) -> Result<(), GitError> {
        let gitignore = dir.join(".gitignore");
        if !gitignore.exists() {
            return Ok(());
        }
        let content = fs::read_to_string(&gitignore)?;
        let filtered: Vec<&str> = content.lines().filter(|l| l.trim() != pattern).collect();
        let new_content = filtered.join("\n");
        fs::write(&gitignore, new_content)?;
        Ok(())
    }
}
