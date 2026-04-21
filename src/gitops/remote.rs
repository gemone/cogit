use super::{GitError, RemoteInfo, Repo};

impl Repo {
    pub fn remotes(&self) -> Result<Vec<RemoteInfo>, GitError> {
        let remotes = self.inner.remotes()?;
        let mut infos = Vec::new();
        for name in remotes.iter().flatten() {
            let url = self.inner.find_remote(name).ok().and_then(|r: git2::Remote<'_>| r.url().map(String::from));
            infos.push(RemoteInfo {
                name: name.to_string(),
                url,
            });
        }
        Ok(infos)
    }

    pub fn fetch(&self, remote: &str) -> Result<(), GitError> {
        let mut remote = self.inner.find_remote(remote)?;
        remote.fetch(&[] as &[&str], None, None)?;
        Ok(())
    }

    pub fn push(&self, remote: &str, refspec: &str) -> Result<(), GitError> {
        let mut remote = self.inner.find_remote(remote)?;
        remote.push(&[refspec], None)?;
        Ok(())
    }
}
