use std::{fs, io, path::PathBuf, time::SystemTime};

/// Watchers the presentation's file.
///
/// This uses polling rather than something fancier like `inotify`. The latter turned out to make
/// code too complex for little added gain. This instead keeps the last modified time for the given
/// path and uses that to determine if it's changed.
pub struct PresentationFileWatcher {
    path: PathBuf,
    last_modification: SystemTime,
}

impl PresentationFileWatcher {
    /// Create a watcher over the given file path.
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        let path = path.into();
        let last_modification = fs::metadata(&path).and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH);
        Self { path, last_modification }
    }

    /// Checker whether this file has modifications.
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
