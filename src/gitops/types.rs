#[derive(Debug, Clone, Default)]
pub struct FileStatus {
    pub branch: String,
    pub ahead: usize,
    pub behind: usize,
    pub staged: Vec<FileEntry>,
    pub unstaged: Vec<FileEntry>,
    pub untracked: Vec<FileEntry>,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub old_path: Option<String>,
    pub status: char,
}

#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
}

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub author_name: String,
    pub author_email: String,
    pub date: String,
    pub subject: String,
    pub graph_prefix: String,
    pub refs: String,  // e.g. "HEAD -> main, tag: v1.0"
}

impl Default for CommitInfo {
    fn default() -> Self {
        Self {
            hash: String::new(),
            short_hash: String::new(),
            author_name: String::new(),
            author_email: String::new(),
            date: String::new(),
            subject: String::new(),
            graph_prefix: String::new(),
            refs: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TagInfo {
    pub name: String,
    pub hash: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CommitDetail {
    pub info: CommitInfo,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub path: String,
    pub branch: Option<String>,
    pub is_main: bool,
}

#[derive(Debug, Clone)]
pub enum RebaseState {
    Idle,
    InProgress { onto: String, done_count: usize, total_count: usize },
}

#[derive(Debug, Clone)]
pub struct ReflogEntry {
    pub hash: String,
    pub short_hash: String,
    pub action: String,
    pub subject: String,
}

/// Action type for a rebase todo entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RebaseAction {
    Pick,
    ReWord,
    Edit,
    Squash,
    FixUp,
    Drop,
}

impl RebaseAction {
    pub fn from_str(s: &str) -> Self {
        match s {
            "reword" | "r" => RebaseAction::ReWord,
            "edit" | "e" => RebaseAction::Edit,
            "squash" | "s" => RebaseAction::Squash,
            "fixup" | "f" => RebaseAction::FixUp,
            "drop" | "d" => RebaseAction::Drop,
            _ => RebaseAction::Pick,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            RebaseAction::Pick => "pick",
            RebaseAction::ReWord => "reword",
            RebaseAction::Edit => "edit",
            RebaseAction::Squash => "squash",
            RebaseAction::FixUp => "fixup",
            RebaseAction::Drop => "drop",
        }
    }

    pub fn short(&self) -> &'static str {
        match self {
            RebaseAction::Pick => "p",
            RebaseAction::ReWord => "r",
            RebaseAction::Edit => "e",
            RebaseAction::Squash => "s",
            RebaseAction::FixUp => "f",
            RebaseAction::Drop => "d",
        }
    }
}

/// A single line in a rebase-todo sequence.
#[derive(Debug, Clone)]
pub struct RebaseTodo {
    pub action: RebaseAction,
    pub hash: String,
    pub short_hash: String,
    pub subject: String,
}

#[derive(Debug, Clone)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
    pub fetch_refspec: String,
    pub push_refspec: String,
}

impl FileStatus {
    pub fn parse(output: &str) -> Self {
        let mut status = FileStatus::default();
        for line in output.lines() {
            if line.starts_with("# branch.head") {
                status.branch = line.split(' ').nth(1).unwrap_or("(detached)").to_string();
            } else if line.starts_with("# branch.ab") {
                let parts: Vec<&str> = line.split(' ').collect();
                for part in parts {
                    if let Some(stripped) = part.strip_prefix('+') {
                        status.ahead = stripped.parse().unwrap_or(0);
                    } else if let Some(stripped) = part.strip_prefix('-') {
                        status.behind = stripped.parse().unwrap_or(0);
                    }
                }
            } else if line.starts_with("1 ") || line.starts_with("2 ") {
                // Changed file
                let chars: Vec<char> = line.chars().collect();
                if chars.len() > 3 {
                    let xy = &line[2..4];
                    let path_start = line.rfind(' ').map(|p| p + 1).unwrap_or(line.len());
                    let path = line[path_start..].to_string();
                    let x = xy.chars().next().unwrap_or('.');
                    let y = xy.chars().nth(1).unwrap_or('.');
                    if x != '.' && x != '?' {
                        status.staged.push(FileEntry {
                            path: path.clone(),
                            old_path: None,
                            status: x,
                        });
                    }
                    if y != '.' && y != '?' {
                        status.unstaged.push(FileEntry {
                            path,
                            old_path: None,
                            status: y,
                        });
                    }
                }
            } else if let Some(stripped) = line.strip_prefix("? ") {
                let path = stripped.to_string();
                status.untracked.push(FileEntry {
                    path,
                    old_path: None,
                    status: '?',
                });
            }
        }
        status
    }
}
