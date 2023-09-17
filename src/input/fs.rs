use std::{fs, io, path::PathBuf, time::SystemTime};

pub struct PresentationFileWatcher {
    path: PathBuf,
    last_modification: SystemTime,
}

impl PresentationFileWatcher {
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        let path = path.into();
        let last_modification = fs::metadata(&path).and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH);
        Self { path, last_modification }
    }

    pub fn has_modifications(&mut self) -> io::Result<bool> {
        let metadata = fs::metadata(&self.path)?;
        let modified_time = metadata.modified()?;
        if modified_time > self.last_modification {
            self.last_modification = modified_time;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
