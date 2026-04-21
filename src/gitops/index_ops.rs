use std::path::Path;
use crate::gitops::{GitError, Repo};

impl Repo {
    pub fn stage_path(&mut self, path: &str) -> Result<(), GitError> {
        let mut index = self.inner.index()?;
        index.add_path(Path::new(path))?;
        index.write()?;
        Ok(())
    }

    pub fn unstage_path(&mut self, path: &str) -> Result<(), GitError> {
        let mut index = self.inner.index()?;
        let head = self.inner.head()?;
        let tree = head.peel_to_tree()?;

        match tree.get_path(Path::new(path)) {
            Ok(tree_entry) => {
                let entry = git2::IndexEntry {
                    ctime: git2::IndexTime::new(0, 0),
                    mtime: git2::IndexTime::new(0, 0),
                    dev: 0,
                    ino: 0,
                    mode: tree_entry.filemode() as u32,
                    uid: 0,
                    gid: 0,
                    file_size: 0,
                    id: tree_entry.id(),
                    flags: 0,
                    flags_extended: 0,
                    path: path.as_bytes().to_vec(),
                };
                index.add(&entry)?;
            }
            Err(e) if e.code() == git2::ErrorCode::NotFound => {
                index.remove_path(Path::new(path))?;
            }
            Err(e) => return Err(e.into()),
        }

        index.write()?;
        Ok(())
    }

    pub fn stage_all(&mut self) -> Result<(), GitError> {
        let mut index = self.inner.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    pub fn unstage_all(&mut self) -> Result<(), GitError> {
        let head = self.inner.head()?;
        let tree = head.peel_to_tree()?;
        let mut index = self.inner.index()?;
        index.read_tree(&tree)?;
        index.write()?;
        Ok(())
    }

    pub fn discard_path(&mut self, path: &str) -> Result<(), GitError> {
        let mut builder = git2::build::CheckoutBuilder::new();
        builder.path(path);
        builder.force();
        self.inner.checkout_head(Some(&mut builder))?;
        Ok(())
    }
}
