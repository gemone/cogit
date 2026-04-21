use super::shell;
use super::{GitError, RemoteInfo, Repo};

impl Repo {
    pub fn remotes(&self) -> Result<Vec<RemoteInfo>, GitError> {
        let output =
            shell::run_git(&mut shell::git_cmd(&self.path).args(["remote"]))?;
        let mut infos = Vec::new();

        for name in output.lines() {
            if name.is_empty() {
                continue;
            }
            let url = shell::run_git(
                &mut shell::git_cmd(&self.path)
                    .args(["config", &format!("remote.{}.url", name)]),
            )
            .ok();
            infos.push(RemoteInfo {
                name: name.to_string(),
                url,
            });
        }

        Ok(infos)
    }

    pub fn fetch(&self, remote: &str) -> Result<(), GitError> {
        let mut cmd = shell::git_cmd(&self.path);
        cmd.args(["fetch", remote]);
        shell::run_git(&mut cmd)?;
        Ok(())
    }

    pub fn push(&self, remote: &str, refspec: &str) -> Result<(), GitError> {
        shell::push(&self.path, remote, refspec)
    }
}
