use crate::error::{CortxError, Result};
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct FileLock {
    lock_path: PathBuf,
    released: bool,
}

impl FileLock {
    pub fn acquire(file_path: &Path) -> Result<Self> {
        let lock_path = lock_path_for(file_path);

        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(_file) => Ok(FileLock {
                lock_path,
                released: false,
            }),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                Err(CortxError::Storage(format!(
                    "entity is locked by another process ({})",
                    lock_path.display()
                )))
            }
            Err(e) => Err(CortxError::Io(e)),
        }
    }

    pub fn release(mut self) -> Result<()> {
        self.do_release()
    }

    fn do_release(&mut self) -> Result<()> {
        if !self.released {
            self.released = true;
            if self.lock_path.exists() {
                fs::remove_file(&self.lock_path)?;
            }
        }
        Ok(())
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = self.do_release();
    }
}

fn lock_path_for(file_path: &Path) -> PathBuf {
    let mut lock = file_path.as_os_str().to_owned();
    lock.push(".lock");
    PathBuf::from(lock)
}
