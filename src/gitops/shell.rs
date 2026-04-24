use anyhow::Result;

use super::Repository;
use crate::gitops::types::*;

#[derive(Debug, Clone)]
pub struct MergePreview {
    pub can_ff: bool,
    pub has_conflicts: bool,
    pub commits_count: usize,
    pub files_changed: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum MergeStrategy {
    FastForward,
    NoFastForward,
    Squash,
}

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

    pub fn checkout_force(&self, refname: &str) -> Result<String> {
        let output = self.git_cmd(&["checkout", "-f", refname])?;
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
            "--all",
            "--graph",
            "--decorate",
            "--pretty=format:%x1f%H|%h|%an|%ae|%aI|%D|%s",
        ])?;
        Self::parse_log_output(&output)
    }

    pub fn log_search(&self, pattern: &str, count: usize) -> Result<Vec<CommitInfo>> {
        let output = self.git_cmd(&[
            "log",
            &format!("-{}", count),
            &format!("--grep={}", pattern),
            "--all",
            "--graph",
            "--decorate",
            "--pretty=format:%x1f%H|%h|%an|%ae|%aI|%D|%s",
        ])?;
        Self::parse_log_output(&output)
    }
    fn parse_log_output(output: &str) -> Result<Vec<CommitInfo>> {
        let sep = '\u{1f}';
        let mut commits = Vec::new();

        for line in output.lines() {
            if let Some(pos) = line.find(sep) {
                let graph_prefix = line[..pos].to_string();
                let data = &line[pos + 1..];
                let parts: Vec<&str> = data.splitn(7, '|').collect();
                if parts.len() >= 7 {
                    commits.push(CommitInfo {
                        hash: parts[0].to_string(),
                        short_hash: parts[1].to_string(),
                        author_name: parts[2].to_string(),
                        author_email: parts[3].to_string(),
                        date: parts[4].to_string(),
                        refs: parts[5].to_string(),
                        subject: parts[6].to_string(),
                        graph_prefix,
                    });
                }
            } else {
                // Pure graph line (branch merge/fork connectors like "|/", "|\", "|\")
                // Keep as connector row for visual continuity
                let trimmed = line.trim();
                if !trimmed.is_empty()
                    && trimmed.chars().all(|c| "|/\\* ".contains(c))
                {
                    commits.push(CommitInfo {
                        graph_prefix: line.to_string(),
                        ..CommitInfo::default()
                    });
                }
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
                graph_prefix: String::new(),
                refs: String::new(),
            }
        } else {
            CommitInfo {
                hash: hash.to_string(),
                short_hash: hash[..7.min(hash.len())].to_string(),
                author_name: String::new(),
                author_email: String::new(),
                date: String::new(),
                subject: first_line.to_string(),
                graph_prefix: String::new(),
                refs: String::new(),
            }
        };
        let body: String = lines.collect::<Vec<_>>().join("\n");
        Ok(CommitDetail { info, body })
    }

    pub fn cherry_pick(&self, hash: &str) -> Result<String> {
        let output = self.git_cmd(&["cherry-pick", hash])?;
        Ok(output)
    }

    pub fn undo(&self) -> Result<String> {
        // Reset to previous reflog entry, keeping working tree intact
        self.git_cmd(&["reset", "--keep", "HEAD@{1}"])?;
        // Return the new HEAD short hash for meaningful notification
        let head = self.git_cmd(&["rev-parse", "--short", "HEAD"])?;
        Ok(head.trim().to_string())
    }

    pub fn revert(&self, hash: &str) -> Result<String> {
        let output = self.git_cmd(&["revert", "--no-edit", hash])?;
        Ok(output.trim().to_string())
    }

    pub fn reflog(&self, count: usize) -> Result<Vec<ReflogEntry>> {
        let output = self.git_cmd(&[
            "reflog",
            &format!("-{}", count),
            "--pretty=format:%x1f%H%x1f%h%x1f%gs%x1f%s",
        ])?;
        let mut entries = Vec::new();
        let sep = '\u{1f}';
        for line in output.lines() {
            let parts: Vec<&str> = line.splitn(4, sep).collect();
            if parts.len() == 4 {
                entries.push(ReflogEntry {
                    hash: parts[0].to_string(),
                    short_hash: parts[1].to_string(),
                    action: parts[2].to_string(),
                    subject: parts[3].to_string(),
                });
            }
        }
        Ok(entries)
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
                let branch = line
                    .strip_prefix("refs/heads/")
                    .unwrap_or(&line[8..])
                    .to_string();
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

    pub(crate) fn git_cmd_with_env(
        &self,
        args: &[&str],
        env: &[(&str, &str)],
    ) -> Result<String> {
        let mut cmd = std::process::Command::new("git");
        cmd.args(args).current_dir(&self.path);
        for (k, v) in env {
            cmd.env(k, v);
        }
        let output = cmd.output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("git {} failed: {}", args.join(" "), stderr);
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub fn preview_merge(&self, branch: &str) -> Result<MergePreview> {
        // Check if fast-forward is possible
        let can_ff = self
            .git_cmd(&["merge-base", "--is-ancestor", "HEAD", branch])
            .is_ok();

        // Get commits that would be merged
        let commits_output = self
            .git_cmd(&["rev-list", "--count", &format!("HEAD..{}", branch)])
            .unwrap_or_default();
        let commits_count = commits_output.trim().parse::<usize>().unwrap_or(0);

        // Get files that would be changed
        let files_output = self
            .git_cmd(&["diff", "--name-only", &format!("HEAD...{}", branch)])
            .unwrap_or_default();
        let files_changed: Vec<String> = files_output
            .lines()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect();

        // Check for conflicts using merge-tree (needs merge-base as first arg)
        let has_conflicts = if commits_count > 0 {
            let base = self
                .git_cmd(&["merge-base", "HEAD", branch])
                .map(|b| b.trim().to_string())
                .unwrap_or_default();
            self.git_cmd(&["merge-tree", &base, "HEAD", branch])
                .map(|output| output.contains("<<"))
                .unwrap_or(false)
        } else {
            false
        };

        Ok(MergePreview {
            can_ff,
            has_conflicts,
            commits_count,
            files_changed,
        })
    }

    pub fn smart_merge(&self, branch: &str, strategy: MergeStrategy) -> Result<String> {
        let mut args = vec!["merge"];

        match strategy {
            MergeStrategy::FastForward => {
                args.push("--ff-only");
            }
            MergeStrategy::NoFastForward => {
                args.push("--no-ff");
            }
            MergeStrategy::Squash => {
                args.push("--squash");
            }
        }

        args.push(branch);

        let output = self.git_cmd(&args)?;

        // For squash, we need to create the commit
        if matches!(strategy, MergeStrategy::Squash) {
            self.git_cmd(&["commit", "-m", &format!("Squash merge {}", branch)])?;
        }

        Ok(output)
    }

    pub fn get_rebase_state(&self) -> Result<RebaseState> {
        // Use git rev-parse --git-path to handle linked worktrees correctly
        let git_dir = self
            .git_cmd(&["rev-parse", "--git-path", "."])
            .map(|p| self.path.join(p.trim()))
            .unwrap_or_else(|_| self.path.join(".git"));
        let rebase_merge_dir = git_dir.join("rebase-merge");
        let rebase_apply_dir = git_dir.join("rebase-apply");

        if rebase_merge_dir.exists() || rebase_apply_dir.exists() {
            let onto_file = if rebase_merge_dir.exists() {
                rebase_merge_dir.join("onto")
            } else {
                rebase_apply_dir.join("onto")
            };

            let onto = std::fs::read_to_string(&onto_file)
                .unwrap_or_default()
                .trim()
                .to_string();

            // Get done commits count
            let done_file = if rebase_merge_dir.exists() {
                rebase_merge_dir.join("done")
            } else {
                rebase_apply_dir.join("done")
            };
            let done_count = std::fs::read_to_string(&done_file)
                .unwrap_or_default()
                .lines()
                .count();

            // Get total commits
            let total_file = if rebase_merge_dir.exists() {
                rebase_merge_dir.join("git-rebase-todo")
            } else {
                rebase_apply_dir.join("git-rebase-todo")
            };
            let todo_count = std::fs::read_to_string(&total_file)
                .unwrap_or_default()
                .lines()
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .count();
            let total_count = done_count + todo_count;

            Ok(RebaseState::InProgress {
                onto: onto[..8.min(onto.len())].to_string(),
                done_count,
                total_count,
            })
        } else {
            Ok(RebaseState::Idle)
        }
    }

    pub fn rebase_continue(&self) -> Result<String> {
        let output = self.git_cmd(&["rebase", "--continue"])?;
        Ok(output)
    }

    pub fn rebase_abort(&self) -> Result<String> {
        let output = self.git_cmd(&["rebase", "--abort"])?;
        Ok(output)
    }

    pub fn rebase_skip(&self) -> Result<String> {
        let output = self.git_cmd(&["rebase", "--skip"])?;
        Ok(output)
    }

    /// Get commits for interactive rebase: all commits between HEAD and `onto`.
    pub fn rebase_get_todo(&self, onto: &str) -> Result<Vec<RebaseTodo>> {
        let output = self.git_cmd(&[
            "log",
            &format!("{}..HEAD", onto),
            "--reverse",
            "--pretty=format:%H|%h|%s",
        ])?;
        let mut todos = Vec::new();
        for line in output.lines() {
            let parts: Vec<&str> = line.splitn(3, '|').collect();
            if parts.len() == 3 {
                todos.push(RebaseTodo {
                    action: RebaseAction::Pick,
                    hash: parts[0].to_string(),
                    short_hash: parts[1].to_string(),
                    subject: parts[2].to_string(),
                });
            }
        }
        Ok(todos)
    }

    /// Execute an interactive rebase with the given todo list.
    /// Uses GIT_SEQUENCE_EDITOR to apply the todo without opening an editor.
    pub fn rebase_interactive(&self, onto: &str, todos: &[RebaseTodo]) -> Result<String> {
        // Build the todo file content
        let todo_content: String = todos
            .iter()
            .map(|t| format!("{} {} {}\n", t.action.as_str(), t.hash, t.subject))
            .collect();

        // Write todo to a temp file and use it as GIT_SEQUENCE_EDITOR
        let dir = std::env::temp_dir().join("cogit-rebase-todo");
        std::fs::create_dir_all(&dir)?;
        let todo_path = dir.join("git-rebase-todo");
        std::fs::write(&todo_path, &todo_content)?;

        let editor_script = format!("cp {} \"$1\"", todo_path.display());

        let output = self.git_cmd_with_env(
            &["rebase", "-i", onto],
            &[("GIT_SEQUENCE_EDITOR", editor_script.as_str())],
        )?;
        Ok(output)
    }

    pub fn remotes(&self) -> Result<Vec<RemoteInfo>> {
        let output = self.git_cmd(&["remote", "-v"])?;
        let mut remotes = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0].to_string();
                let url = parts[1].to_string();
                let _direction = parts.get(2).map(|s| s.trim_matches('(')).unwrap_or("fetch");

                if seen.insert(name.clone()) {
                    let fetch_refspec = format!("+refs/heads/*:refs/remotes/{}/*", name);
                    let push_refspec = format!("refs/heads/*:refs/remotes/{}/*", name);

                    remotes.push(RemoteInfo {
                        name,
                        url,
                        fetch_refspec,
                        push_refspec,
                    });
                }
            }
        }

        Ok(remotes)
    }

    pub fn add_remote(&self, name: &str, url: &str) -> Result<String> {
        let output = self.git_cmd(&["remote", "add", name, url])?;
        Ok(output)
    }

    pub fn remove_remote(&self, name: &str) -> Result<String> {
        let output = self.git_cmd(&["remote", "remove", name])?;
        Ok(output)
    }

    pub fn rename_remote(&self, old: &str, new: &str) -> Result<String> {
        let output = self.git_cmd(&["remote", "rename", old, new])?;
        Ok(output)
    }

    pub fn fetch_remote(&self, name: &str) -> Result<String> {
        let output = self.git_cmd(&["fetch", name])?;
        Ok(output)
    }

    pub fn checkout_remote_branch(&self, remote_branch: &str) -> Result<String> {
        // Extract branch name from "remotes/origin/feature-x" -> "feature-x"
        let branch_name = remote_branch
            .strip_prefix("remotes/")
            .unwrap_or(remote_branch)
            .split('/')
            .skip(1)
            .collect::<Vec<_>>()
            .join("/");

        // Check if local branch already exists
        let local_exists = self
            .git_cmd(&["rev-parse", "--verify", &branch_name])
            .is_ok();

        if local_exists {
            // Reset local branch to remote
            let remote_ref = remote_branch
                .strip_prefix("remotes/")
                .unwrap_or(remote_branch);
            self.git_cmd(&["branch", "-f", &branch_name, remote_ref])?;
            self.git_cmd(&["checkout", &branch_name])?;
            Ok(format!(
                "Reset and checked out existing branch: {}",
                branch_name
            ))
        } else {
            // Create new tracking branch
            let output = self.git_cmd(&["checkout", "-b", &branch_name, remote_branch])?;
            Ok(output)
        }
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
        let is_duplicate = content.lines().any(|line| line.trim() == pattern_trimmed);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_repo(dir_name: &str) -> (Repository, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("cogit-test-{}", dir_name));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let repo = Repository::open(&dir).unwrap();
        // Use -b main to ensure consistent branch name across git versions
        repo.git_cmd(&["init", "-b", "main"]).unwrap();
        repo.git_cmd(&["config", "user.name", "Test"]).unwrap();
        repo.git_cmd(&["config", "user.email", "test@test.com"])
            .unwrap();
        fs::write(dir.join("file.txt"), "initial\n").unwrap();
        repo.git_cmd(&["add", "."]).unwrap();
        repo.git_cmd(&["commit", "-m", "initial"]).unwrap();
        (repo, dir)
    }

    fn get_default_branch_name() -> String {
        // Query the default branch name from git config or fallback to "main"
        std::process::Command::new("git")
            .args(["config", "--global", "init.defaultBranch"])
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "main".to_string())
    }

    #[test]
    fn test_current_branch() {
        let (repo, _dir) = setup_test_repo("current-branch");
        let branch = repo.current_branch().unwrap();
        let expected = get_default_branch_name();
        assert_eq!(branch, expected);
    }

    #[test]
    fn test_branches() {
        let (repo, _dir) = setup_test_repo("branches");
        let branches = repo.branches().unwrap();
        assert!(!branches.is_empty());
        let expected = get_default_branch_name();
        let main_branch = branches.iter().find(|b| b.name == expected);
        assert!(main_branch.is_some());
        assert!(main_branch.unwrap().is_current);
    }

    #[test]
    fn test_log_returns_commit_data() {
        let (repo, dir) = setup_test_repo("log-data");

        fs::write(dir.join("file.txt"), "second\n").unwrap();
        repo.stage("file.txt").unwrap();
        repo.git_cmd(&["commit", "-m", "second commit"]).unwrap();

        let commits = repo.log(10).unwrap();
        assert!(!commits.is_empty(), "log should return commit entries");

        let latest = &commits[0];
        assert_eq!(latest.subject, "second commit");
        assert!(!latest.hash.is_empty(), "commit hash should be populated");
        assert!(
            !latest.short_hash.is_empty(),
            "short hash should be populated"
        );
        assert_eq!(latest.author_name, "Test");
        assert_eq!(latest.author_email, "test@test.com");
        assert!(!latest.date.is_empty(), "commit date should be populated");
    }

    #[test]
    fn test_checkout() {
        let (repo, _dir) = setup_test_repo("checkout");
        repo.create_branch("feature").unwrap();
        repo.checkout("feature").unwrap();
        let branch = repo.current_branch().unwrap();
        assert_eq!(branch, "feature");
    }

    #[test]
    fn test_stage_and_unstage() {
        let (repo, dir) = setup_test_repo("stage-unstage");
        fs::write(dir.join("file.txt"), "modified\n").unwrap();
        repo.stage("file.txt").unwrap();
        let status = repo.status().unwrap();
        assert!(!status.staged.is_empty());

        repo.unstage("file.txt").unwrap();
        let status = repo.status().unwrap();
        assert!(status.staged.is_empty());
        assert!(!status.unstaged.is_empty());
    }

    #[test]
    fn test_undo_resets_to_previous_commit() {
        let (repo, dir) = setup_test_repo("undo");
        let head_before = repo
            .git_cmd(&["rev-parse", "HEAD"])
            .unwrap()
            .trim()
            .to_string();

        // Create a second commit
        fs::write(dir.join("file.txt"), "second\n").unwrap();
        repo.stage("file.txt").unwrap();
        repo.git_cmd(&["commit", "-m", "second"]).unwrap();
        let head_after = repo
            .git_cmd(&["rev-parse", "HEAD"])
            .unwrap()
            .trim()
            .to_string();
        assert_ne!(head_before, head_after, "commit should advance HEAD");

        // Undo should move HEAD back
        repo.undo().unwrap();
        let head_after_undo = repo
            .git_cmd(&["rev-parse", "HEAD"])
            .unwrap()
            .trim()
            .to_string();
        assert_eq!(
            head_before, head_after_undo,
            "undo should restore previous HEAD"
        );
    }

    #[test]
    fn test_revert_creates_new_commit() {
        let (repo, dir) = setup_test_repo("revert");

        // Create a second commit
        fs::write(dir.join("file.txt"), "second\n").unwrap();
        repo.stage("file.txt").unwrap();
        repo.git_cmd(&["commit", "-m", "second"]).unwrap();
        let head_second = repo
            .git_cmd(&["rev-parse", "HEAD"])
            .unwrap()
            .trim()
            .to_string();

        // Revert the second commit
        repo.revert(&head_second).unwrap();
        let head_after_revert = repo
            .git_cmd(&["rev-parse", "HEAD"])
            .unwrap()
            .trim()
            .to_string();
        assert_ne!(
            head_second, head_after_revert,
            "revert should create a new commit"
        );

        // File content should be back to initial state
        let content = fs::read_to_string(dir.join("file.txt")).unwrap();
        assert_eq!(content, "initial\n", "revert should restore file content");
    }
}
