use super::shell;
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
        let output = shell::run_git(&mut shell::git_cmd(&self.path).args([
            "status",
            "--porcelain",
            "--no-renames",
        ]))?;

        let mut files = Vec::new();
        for line in output.lines() {
            if line.len() < 4 {
                continue;
            }
            let x = line.as_bytes()[0];
            let y = line.as_bytes()[1];
            let path = &line[3..];

            let file_status = match (x, y) {
                (b'?' | b'!', _) | (_, b'?' | b'!') => {
                    // Untracked or ignored
                    if x == b'!' || y == b'!' {
                        FileStatus::Ignored
                    } else {
                        FileStatus::Untracked
                    }
                }
                (b'U', _) | (_, b'U') | (b'A', b'A') | (b'D', b'D') => FileStatus::Conflicted,
                (b'A', _) | (b'C', _) => FileStatus::StagedNew,
                (b'M', _) => FileStatus::StagedModified,
                (_, b'M') | (_, b'D') | (_, b'A') => {
                    // Working tree change — if index also staged, already handled above
                    FileStatus::Modified
                }
                // Other cases: renamed (R), deleted in index (D)
                (b'D', _) => FileStatus::StagedModified,
                (b'R', _) => FileStatus::StagedModified,
                _ => continue,
            };

            files.push(WorktreeFile {
                path: path.to_string(),
                status: file_status,
            });
        }

        Ok(files)
    }
}
