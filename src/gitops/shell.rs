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

    pub fn commit(&self, message: &str) -> Result<String> {
        let output = self.git_cmd(&["commit", "-m", message])?;
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
}
