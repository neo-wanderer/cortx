use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub struct TestVault {
    pub dir: TempDir,
}

impl TestVault {
    pub fn new() -> Self {
        let dir = TempDir::new().unwrap();
        for folder in &[
            "0_Inbox",
            "1_Projects",
            "1_Projects/tasks",
            "2_Areas",
            "3_Resources",
            "3_Resources/notes",
            "4_Archive",
            "5_People",
            "5_Companies",
        ] {
            fs::create_dir_all(dir.path().join(folder)).unwrap();
        }
        TestVault { dir }
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn write_file(&self, rel_path: &str, content: &str) {
        let full_path = self.dir.path().join(rel_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(full_path, content).unwrap();
    }

    pub fn read_file(&self, rel_path: &str) -> String {
        fs::read_to_string(self.dir.path().join(rel_path)).unwrap()
    }

    pub fn file_exists(&self, rel_path: &str) -> bool {
        self.dir.path().join(rel_path).exists()
    }
}
