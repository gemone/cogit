use anyhow::Result;

use super::Repository;
use crate::gitops::types::*;

impl Repository {
    pub fn status(&self) -> Result<FileStatus> {
        let output = self.git_cmd(&["status", "--porcelain=v2", "--branch"])?;
        Ok(FileStatus::parse(&output))
    }

    pub fn stage(&self, path: &str) -> Result<()> {
        self.git_cmd(&["add", path])?;
        Ok(())
    }

    pub fn stage_all(&self) -> Result<()> {
        self.git_cmd(&["add", "--all"])?;
        Ok(())
    }

    pub fn unstage(&self, path: &str) -> Result<()> {
        self.git_cmd(&["reset", "HEAD", "--", path])?;
        Ok(())
    }

    pub fn unstage_all(&self) -> Result<()> {
        self.git_cmd(&["reset", "HEAD"])?;
        Ok(())
    }

    pub fn reset(&self, mode: &str, path: &str) -> Result<()> {
        // mode: "soft", "hard", or "mixed"
        // path: empty string means whole repo, otherwise specific path
        let mut args = vec!["reset"];
        match mode {
            "soft" => args.push("--soft"),
            "hard" => args.push("--hard"),
            "mixed" => args.push("--mixed"),
            _ => args.push("--mixed"), // default to mixed
        }
        if !path.is_empty() {
            args.push("--");
            args.push(path);
        }
        self.git_cmd(&args)?;
        Ok(())
    }

    pub fn commit(&self, message: &str) -> Result<String> {
        let output = self.git_cmd(&["commit", "-m", message])?;
        Ok(output)
    }

    #[allow(dead_code)]
    pub fn commit_no_verify(&self, message: &str) -> Result<String> {
        let output = self.git_cmd(&["commit", "--no-verify", "-m", message])?;
        Ok(output)
    }

    pub fn wip_commit(&self) -> Result<String> {
        let output = self.git_cmd(&["commit", "-m", "WIP", "--no-verify"])?;
        Ok(output)
    }

    pub fn amend_commit(&self, message: Option<&str>) -> Result<String> {
        let mut args = vec!["commit", "--amend"];
        if let Some(msg) = message {
            args.push("-m");
            args.push(msg);
        } else {
            args.push("--no-edit");
        }
        let output = self.git_cmd(&args)?;
        Ok(output)
    }

    pub fn discard(&self, path: &str) -> Result<()> {
        self.git_cmd(&["checkout", "--", path])?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn push(&self, remote: &str, branch: &str) -> Result<String> {
        let output = self.git_cmd(&["push", remote, branch])?;
        Ok(output)
    }

    pub fn push_current(&self) -> Result<String> {
        let branch = self.current_branch()?;
        let output = self.git_cmd(&["push", "-u", "origin", &branch])?;
        Ok(output)
    }

    #[allow(dead_code)]
    pub fn pull(&self, remote: &str, branch: &str) -> Result<String> {
        let output = self.git_cmd(&["pull", remote, branch])?;
        Ok(output)
    }

    pub fn pull_current(&self) -> Result<String> {
        let branch = self.current_branch()?;
        let output = self.git_cmd(&["pull", "origin", &branch])?;
        Ok(output)
    }

    pub fn pull_rebase_current(&self) -> Result<String> {
        let branch = self.current_branch()?;
        let output = self.git_cmd(&["pull", "--rebase", "origin", &branch])?;
        Ok(output)
    }

    #[allow(dead_code)]
    pub fn fetch(&self, remote: &str) -> Result<String> {
        let output = self.git_cmd(&["fetch", remote])?;
        Ok(output)
    }

    pub fn fetch_all(&self) -> Result<String> {
        let output = self.git_cmd(&["fetch", "--all"])?;
        Ok(output)
    }

    pub fn merge(&self, branch: &str) -> Result<String> {
        let output = self.git_cmd(&["merge", branch])?;
        Ok(output)
    }

    pub fn rebase(&self, branch: &str) -> Result<String> {
        let output = self.git_cmd(&["rebase", branch])?;
        Ok(output)
    }

    pub fn checkout(&self, refname: &str) -> Result<String> {
        let output = self.git_cmd(&["checkout", refname])?;
        Ok(output)
    }

    pub fn create_branch(&self, name: &str) -> Result<String> {
        let output = self.git_cmd(&["branch", name])?;
        Ok(output)
    }

    pub fn delete_branch(&self, name: &str) -> Result<String> {
        let output = self.git_cmd(&["branch", "-d", name])?;
        Ok(output)
    }

    pub fn rename_branch(&self, old_name: &str, new_name: &str) -> Result<String> {
        let output = self.git_cmd(&["branch", "-m", old_name, new_name])?;
        Ok(output)
    }

    pub fn current_branch(&self) -> Result<String> {
        let output = self.git_cmd(&["rev-parse", "--abbrev-ref", "HEAD"])?;
        Ok(output.trim().to_string())
    }

    pub fn branches(&self) -> Result<Vec<BranchInfo>> {
        let output = self.git_cmd(&["branch", "-a", "--list"])?;
        let _current = self.current_branch()?;
        let mut branches = Vec::new();
        for line in output.lines() {
            let line = line.trim_start();
            let is_current = line.starts_with('*');
            let name = line.trim_start_matches('*').trim().to_string();
            if name.is_empty() || name.starts_with('(') {
                continue;
            }
            let is_remote = name.starts_with("remotes/");
            branches.push(BranchInfo {
                name,
                is_current,
                is_remote,
            });
        }
        Ok(branches)
    }

    pub fn log(&self, count: usize) -> Result<Vec<CommitInfo>> {
        let output = self.git_cmd(&[
            "log",
            &format!("-{}", count),
            "--pretty=format:%H|%h|%an|%ae|%aI|%s",
        ])?;
        let mut commits = Vec::new();
        for line in output.lines() {
            let parts: Vec<&str> = line.splitn(6, '|').collect();
            if parts.len() == 6 {
                commits.push(CommitInfo {
                    hash: parts[0].to_string(),
                    short_hash: parts[1].to_string(),
                    author_name: parts[2].to_string(),
                    author_email: parts[3].to_string(),
                    date: parts[4].to_string(),
                    subject: parts[5].to_string(),
                });
            }
        }
        Ok(commits)
    }

    pub fn log_search(&self, pattern: &str, count: usize) -> Result<Vec<CommitInfo>> {
        let output = self.git_cmd(&[
            "log",
            &format!("-{}", count),
            &format!("--grep={}", pattern),
            "--pretty=format:%H|%h|%an|%ae|%aI|%s",
        ])?;
        let mut commits = Vec::new();
        for line in output.lines() {
            let parts: Vec<&str> = line.splitn(6, '|').collect();
            if parts.len() == 6 {
                commits.push(CommitInfo {
                    hash: parts[0].to_string(),
                    short_hash: parts[1].to_string(),
                    author_name: parts[2].to_string(),
                    author_email: parts[3].to_string(),
                    date: parts[4].to_string(),
                    subject: parts[5].to_string(),
                });
            }
        }
        Ok(commits)
    }

    pub fn show_commit(&self, hash: &str) -> Result<CommitDetail> {
        let output = self.git_cmd(&[
            "show",
            "--pretty=format:%H|%h|%an|%ae|%aI|%s%n%b",
            "--stat",
            hash,
        ])?;
        let mut lines = output.lines();
        let first_line = lines.next().unwrap_or("");
        let parts: Vec<&str> = first_line.splitn(6, '|').collect();
        let info = if parts.len() == 6 {
            CommitInfo {
                hash: parts[0].to_string(),
                short_hash: parts[1].to_string(),
                author_name: parts[2].to_string(),
                author_email: parts[3].to_string(),
                date: parts[4].to_string(),
                subject: parts[5].to_string(),
            }
        } else {
            CommitInfo {
                hash: hash.to_string(),
                short_hash: hash[..7.min(hash.len())].to_string(),
                author_name: String::new(),
                author_email: String::new(),
                date: String::new(),
                subject: first_line.to_string(),
            }
        };
        let body: String = lines.collect::<Vec<_>>().join("\n");
        Ok(CommitDetail { info, body })
    }

    pub fn cherry_pick(&self, hash: &str) -> Result<String> {
        let output = self.git_cmd(&["cherry-pick", hash])?;
        Ok(output)
    }

    pub fn tag_list(&self) -> Result<Vec<TagInfo>> {
        let output = self.git_cmd(&[
            "tag",
            "-l",
            "--format=%(refname:short)|%(objectname)|%(contents:subject)",
        ])?;
        let mut tags = Vec::new();
        for line in output.lines() {
            let parts: Vec<&str> = line.splitn(3, '|').collect();
            if !parts.is_empty() && !parts[0].is_empty() {
                tags.push(TagInfo {
                    name: parts[0].to_string(),
                    hash: parts.get(1).unwrap_or(&"").to_string(),
                    message: parts.get(2).map(|s| s.to_string()),
                });
            }
        }
        Ok(tags)
    }

    pub fn tag_create(&self, name: &str, hash: &str, message: Option<&str>) -> Result<String> {
        let mut args = vec!["tag"];
        if let Some(msg) = message {
            args.push("-a");
            args.push(name);
            args.push("-m");
            args.push(msg);
        } else {
            args.push(name);
        }
        if !hash.is_empty() {
            args.push(hash);
        }
        let output = self.git_cmd(&args)?;
        Ok(output)
    }

    pub fn tag_delete(&self, name: &str) -> Result<String> {
        let output = self.git_cmd(&["tag", "-d", name])?;
        Ok(output)
    }

    pub fn file_diff(&self, path: &str) -> Result<String> {
        let output = self.git_cmd(&["diff", path]).unwrap_or_default();
        if output.is_empty() {
            // Try cached diff (staged file)
            let cached = self
                .git_cmd(&["diff", "--cached", path])
                .unwrap_or_default();
            if cached.is_empty() {
                Ok(format!("(no changes: {})", path))
            } else {
                Ok(cached)
            }
        } else {
            Ok(output)
        }
    }

    pub fn diff_refs(&self, from: &str, to: &str) -> Result<String> {
        let output = self.git_cmd(&["diff", &format!("{}..{}", from, to)])?;
        Ok(output)
    }

    pub fn worktree_list(&self) -> Result<Vec<WorktreeInfo>> {
        let output = self.git_cmd(&["worktree", "list", "--porcelain"])?;
        let mut worktrees = Vec::new();
        let mut current = WorktreeInfo {
            path: String::new(),
            branch: None,
            is_main: false,
        };
        let mut in_worktree = false;

        for line in output.lines() {
            if line.starts_with("worktree ") {
                if !current.path.is_empty() {
                    worktrees.push(current);
                }
                current = WorktreeInfo {
                    path: line.strip_prefix("worktree ").unwrap_or(line).to_string(),
                    branch: None,
                    is_main: false,
                };
                in_worktree = true;
            } else if line.starts_with("HEAD ") && in_worktree {
                // HEAD is present only for non-main worktrees
                current.is_main = false;
            } else if line.starts_with("branch ") && in_worktree {
                let branch = line.strip_prefix("refs/heads/").unwrap_or(&line[8..]).to_string();
                current.branch = Some(branch);
            } else if line.trim().is_empty() && in_worktree {
                // Empty line marks end of worktree entry
                if !current.path.is_empty() {
                    worktrees.push(current);
                    current = WorktreeInfo {
                        path: String::new(),
                        branch: None,
                        is_main: false,
                    };
                }
                in_worktree = false;
            }
        }
        if !current.path.is_empty() {
            worktrees.push(current);
        }

        // First worktree is always the main one
        if !worktrees.is_empty() {
            worktrees[0].is_main = true;
        }

        Ok(worktrees)
    }

    pub fn worktree_create(&self, path: &str, branch: &str) -> Result<String> {
        let output = self.git_cmd(&["worktree", "add", path, branch])?;
        Ok(output)
    }

    pub fn worktree_remove(&self, path: &str) -> Result<String> {
        let output = self.git_cmd(&["worktree", "remove", path])?;
        Ok(output)
    }

    pub(crate) fn git_cmd(&self, args: &[&str]) -> Result<String> {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(&self.path)
            .output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("git {} failed: {}", args.join(" "), stderr);
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub fn gitignore_read(&self) -> Result<String> {
        let gitignore_path = self.path.join(".gitignore");
        if gitignore_path.exists() {
            Ok(std::fs::read_to_string(&gitignore_path)?)
        } else {
            Ok(String::new())
        }
    }

    pub fn gitignore_add(&self, pattern: &str) -> Result<()> {
        let gitignore_path = self.path.join(".gitignore");
        let mut content = if gitignore_path.exists() {
            std::fs::read_to_string(&gitignore_path)?
        } else {
            String::new()
        };
        // Append pattern if not already present (check exact line match after trimming)
        let pattern_trimmed = pattern.trim();
        let is_duplicate = content
            .lines()
            .any(|line| line.trim() == pattern_trimmed);
        if !is_duplicate {
            if !content.ends_with('\n') && !content.is_empty() {
                content.push('\n');
            }
            content.push_str(pattern);
            content.push('\n');
            std::fs::write(&gitignore_path, content)?;
        }
        Ok(())
    }

    pub fn gitignore_remove(&self, pattern: &str) -> Result<()> {
        let gitignore_path = self.path.join(".gitignore");
        if !gitignore_path.exists() {
            return Ok(());
        }
        let content = std::fs::read_to_string(&gitignore_path)?;
        let has_trailing_newline = content.ends_with('\n');
        let new_content: String = content
            .lines()
            .filter(|line| line.trim() != pattern.trim())
            .collect::<Vec<_>>()
            .join("\n");
        // Preserve original trailing newline
        let new_content = if has_trailing_newline && !new_content.is_empty() {
            new_content + "\n"
        } else {
            new_content
        };
        // Compare normalized representations before writing
        let content_normalized = content.trim_end();
        let new_content_normalized = new_content.trim_end();
        if new_content_normalized != content_normalized {
            std::fs::write(&gitignore_path, new_content)?;
        }
        Ok(())
    }
}
