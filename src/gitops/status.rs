use super::{GitError, Repo};

#[derive(Debug, Clone)]
pub enum FileStatus {
    Untracked,
    Modified,
    StagedNew,
    StagedModified,
    Conflicted,
    Ignored,
}

#[derive(Debug, Clone)]
pub struct WorktreeFile {
    pub path: String,
    pub status: FileStatus,
}

impl Repo {
    pub fn status(&self) -> Result<Vec<WorktreeFile>, GitError> {
        let mut opts = git2::StatusOptions::new();
        opts.include_untracked(true)
            .renames_head_to_index(true)
            .renames_index_to_workdir(true);

        let statuses = self.inner.statuses(Some(&mut opts))?;
        let mut files = Vec::new();

        for entry in statuses.iter() {
            let path: &str = match entry.path() {
                Some(p) => p,
                None => continue,
            };
            let status = entry.status();

            let file_status = if status.contains(git2::Status::CONFLICTED) {
                FileStatus::Conflicted
            } else if status.contains(git2::Status::IGNORED) {
                FileStatus::Ignored
            } else if status.contains(git2::Status::INDEX_NEW)
                || status.contains(git2::Status::INDEX_MODIFIED)
                || status.contains(git2::Status::INDEX_RENAMED)
                || status.contains(git2::Status::INDEX_DELETED)
            {
                if status.contains(git2::Status::INDEX_NEW) {
                    FileStatus::StagedNew
                } else {
                    FileStatus::StagedModified
                }
            } else if status.contains(git2::Status::WT_NEW) {
                FileStatus::Untracked
            } else if status.contains(git2::Status::WT_MODIFIED)
                || status.contains(git2::Status::WT_DELETED)
                || status.contains(git2::Status::WT_RENAMED)
            {
                FileStatus::Modified
            } else {
                continue;
            };

            files.push(WorktreeFile {
                path: path.to_string(),
                status: file_status,
            });
        }

        Ok(files)
    }
}
