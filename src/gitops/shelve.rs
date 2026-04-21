use super::shell;
use super::{GitError, Repo, ShelveInfo};
use std::fs;

impl Repo {
    fn shelves_dir(&self) -> std::path::PathBuf {
        self.path.join(".cogit").join("shelves")
    }

    fn ensure_shelves_dir(&self) -> Result<(), GitError> {
        let dir = self.shelves_dir();
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }
        Ok(())
    }

    pub fn shelve(&self, name: &str, paths: &[&str]) -> Result<(), GitError> {
        self.ensure_shelves_dir()?;

        // Generate diff using git CLI
        let patch = shell::diff_to_file(&self.path, paths)?;

        let file_path = self.shelves_dir().join(format!("{}.patch", name));
        fs::write(&file_path, &patch)?;

        // Reset the working tree for the given paths
        shell::checkout_paths(&self.path, paths)?;

        Ok(())
    }

    pub fn unshelve(&self, name: &str) -> Result<(), GitError> {
        let file_path = self.shelves_dir().join(format!("{}.patch", name));
        if !file_path.exists() {
            return Err(GitError::ShelveNotFound(name.to_string()));
        }
        let patch = fs::read(&file_path)?;
        shell::apply_patch(&self.path, &patch)?;
        fs::remove_file(&file_path)?;
        Ok(())
    }

    pub fn delete_shelve(&self, name: &str) -> Result<(), GitError> {
        let file_path = self.shelves_dir().join(format!("{}.patch", name));
        if !file_path.exists() {
            return Err(GitError::ShelveNotFound(name.to_string()));
        }
        fs::remove_file(&file_path)?;
        Ok(())
    }

    pub fn list_shelves(&self) -> Result<Vec<ShelveInfo>, GitError> {
        let dir = self.shelves_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut shelves = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(stem) = name.strip_suffix(".patch") {
                let meta = entry.metadata().ok();
                let created_at = meta
                    .and_then(|m| m.created().ok())
                    .map(|t| {
                        chrono::DateTime::<chrono::Local>::from(t)
                            .format("%Y-%m-%d %H:%M:%S")
                            .to_string()
                    });
                shelves.push(ShelveInfo {
                    name: stem.to_string(),
                    created_at,
                });
            }
        }
        Ok(shelves)
    }
}
