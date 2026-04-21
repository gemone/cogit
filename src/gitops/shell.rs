use std::path::Path;
use std::process::Command;

use super::GitError;

pub(crate) fn git_cmd(repo_path: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(repo_path);
    cmd
}

pub(crate) fn run_git(cmd: &mut Command) -> Result<String, GitError> {
    let output = cmd.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::Other(stderr.to_string()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn push(repo_path: &Path, remote: &str, refspec: &str) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["push", remote, refspec]);
    run_git(&mut cmd)?;
    Ok(())
}

/// Returns `true` if the merge was a fast-forward.
pub fn merge(repo_path: &Path, branch: &str) -> Result<bool, GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["merge", "--ff-only", branch]);
    let res = run_git(&mut cmd);
    match res {
        Ok(_) => Ok(true),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("not possible") || msg.contains("Cannot fast-forward") {
                // Try a normal merge
                let mut cmd2 = git_cmd(repo_path);
                cmd2.args(["merge", branch]);
                run_git(&mut cmd2)?;
                Ok(false)
            } else {
                Err(e)
            }
        }
    }
}

pub fn rebase(repo_path: &Path, branch: &str) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["rebase", branch]);
    run_git(&mut cmd)?;
    Ok(())
}

pub fn rebase_continue(repo_path: &Path) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["rebase", "--continue"]);
    run_git(&mut cmd)?;
    Ok(())
}

pub fn rebase_abort(repo_path: &Path) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["rebase", "--abort"]);
    run_git(&mut cmd)?;
    Ok(())
}

pub fn rebase_skip(repo_path: &Path) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["rebase", "--skip"]);
    run_git(&mut cmd)?;
    Ok(())
}

pub fn pull(repo_path: &Path, remote: &str, branch: &str) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["pull", remote, branch]);
    run_git(&mut cmd)?;
    Ok(())
}

pub fn stash_save(repo_path: &Path, msg: &str, include_untracked: bool) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["stash", "push", "-m", msg]);
    if include_untracked {
        cmd.arg("--include-untracked");
    }
    run_git(&mut cmd)?;
    Ok(())
}

pub fn stash_pop(repo_path: &Path, index: usize) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["stash", "pop", &format!("stash@{{{}}}", index)]);
    run_git(&mut cmd)?;
    Ok(())
}

pub fn stash_apply(repo_path: &Path, index: usize) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["stash", "apply", &format!("stash@{{{}}}", index)]);
    run_git(&mut cmd)?;
    Ok(())
}

pub fn stash_drop(repo_path: &Path, index: usize) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["stash", "drop", &format!("stash@{{{}}}", index)]);
    run_git(&mut cmd)?;
    Ok(())
}

pub fn stash_list(repo_path: &Path) -> Result<String, GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["stash", "list"]);
    run_git(&mut cmd)
}

pub fn checkout(repo_path: &Path, name: &str) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["checkout", name]);
    run_git(&mut cmd)?;
    Ok(())
}

pub fn create_branch(repo_path: &Path, name: &str, base: &str) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["branch", name, base]);
    run_git(&mut cmd)?;
    Ok(())
}

pub fn delete_branch(repo_path: &Path, name: &str, force: bool) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    let flag = if force { "-D" } else { "-d" };
    cmd.args(["branch", flag, name]);
    run_git(&mut cmd)?;
    Ok(())
}

pub fn set_upstream(repo_path: &Path, local: &str, remote: &str) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["branch", "--set-upstream-to", remote, local]);
    run_git(&mut cmd)?;
    Ok(())
}

pub fn cherry_pick(repo_path: &Path, oid: &str) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["cherry-pick", oid]);
    run_git(&mut cmd)?;
    Ok(())
}

pub fn cherry_pick_abort(repo_path: &Path) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["cherry-pick", "--abort"]);
    run_git(&mut cmd)?;
    Ok(())
}

pub fn cherry_pick_continue(repo_path: &Path) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["cherry-pick", "--continue"]);
    // --no-edit to avoid opening editor
    cmd.arg("--no-edit");
    run_git(&mut cmd)?;
    Ok(())
}

pub fn commit(repo_path: &Path, message: &str) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["commit", "-m", message]);
    run_git(&mut cmd)?;
    Ok(())
}

pub fn commit_amend(repo_path: &Path, message: &str) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["commit", "--amend", "-m", message]);
    run_git(&mut cmd)?;
    Ok(())
}

pub fn stage_path(repo_path: &Path, path: &str) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["add", "--", path]);
    run_git(&mut cmd)?;
    Ok(())
}

pub fn unstage_path(repo_path: &Path, path: &str) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["reset", "HEAD", "--", path]);
    run_git(&mut cmd)?;
    Ok(())
}

pub fn stage_all(repo_path: &Path) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["add", "--all"]);
    run_git(&mut cmd)?;
    Ok(())
}

pub fn unstage_all(repo_path: &Path) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["reset", "HEAD"]);
    run_git(&mut cmd)?;
    Ok(())
}

pub fn discard_path(repo_path: &Path, path: &str) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["checkout", "HEAD", "--", path]);
    run_git(&mut cmd)?;
    Ok(())
}

pub fn diff_for_file(repo_path: &Path, path: &str) -> Result<String, GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["diff", "--", path]);
    run_git(&mut cmd)
}

pub fn diff_staged_for_file(repo_path: &Path, path: &str) -> Result<String, GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["diff", "--cached", "--", path]);
    run_git(&mut cmd)
}

pub fn diff_to_file(repo_path: &Path, paths: &[&str]) -> Result<String, GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["diff"]);
    for p in paths {
        cmd.arg("--").arg(p);
    }
    run_git(&mut cmd)
}

pub fn list_branches(repo_path: &Path) -> Result<String, GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["branch", "--format=%(refname:short)"]);
    run_git(&mut cmd)
}

pub fn list_remote_branches(repo_path: &Path) -> Result<String, GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["branch", "-r", "--format=%(refname:short)"]);
    run_git(&mut cmd)
}

pub fn list_tags(repo_path: &Path) -> Result<String, GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["tag"]);
    run_git(&mut cmd)
}

pub fn log_oneline(repo_path: &Path, n: usize) -> Result<String, GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["log", &format!("-{}", n), "--oneline"]);
    run_git(&mut cmd)
}

pub fn apply_patch(repo_path: &Path, patch: &[u8]) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["apply"]);
    cmd.stdin(std::process::Stdio::piped());
    let mut child = cmd.spawn()?;
    use std::io::Write;
    child.stdin.as_mut().unwrap().write_all(patch)?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::Other(stderr.to_string()));
    }
    Ok(())
}

pub fn checkout_paths(repo_path: &Path, paths: &[&str]) -> Result<(), GitError> {
    let mut cmd = git_cmd(repo_path);
    cmd.args(["checkout", "HEAD", "--"]);
    for p in paths {
        cmd.arg(p);
    }
    run_git(&mut cmd)?;
    Ok(())
}

/// Check if a rebase is in progress by looking at .git directory state.
pub fn is_rebasing(repo_path: &Path) -> bool {
    let git_dir = find_git_dir(repo_path);
    git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists()
}

/// Check if a merge is in progress by looking at .git/MERGE_HEAD.
pub fn is_merging(repo_path: &Path) -> bool {
    let git_dir = find_git_dir(repo_path);
    git_dir.join("MERGE_HEAD").exists()
}

fn find_git_dir(repo_path: &Path) -> std::path::PathBuf {
    // Try common .git location; for worktrees this may differ
    // but for typical usage this is sufficient
    let git_path = repo_path.join(".git");
    if git_path.is_dir() {
        git_path
    } else if git_path.exists() {
        // .git file (worktree) — read the gitdir pointer
        if let Ok(content) = std::fs::read_to_string(&git_path) {
            if let Some(line) = content.lines().next() {
                if let Some(gitdir) = line.strip_prefix("gitdir: ") {
                    return std::path::PathBuf::from(gitdir.trim());
                }
            }
        }
        repo_path.join(".git")
    } else {
        repo_path.join(".git")
    }
}
