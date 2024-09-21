use crate::{
    media::{image::Image, printer::RegisterImageError, register::ImageRegistry},
    theme::{LoadThemeError, PresentationTheme},
};
use std::{
    collections::HashMap,
    fs, io, mem,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender, channel},
    },
    thread,
    time::{Duration, SystemTime},
};

const LOOP_INTERVAL: Duration = Duration::from_millis(250);

/// Manages resources pulled from the filesystem such as images.
///
/// All resources are cached so once a specific resource is loaded, looking it up with the same
/// path will involve an in-memory lookup.
pub struct Resources {
    base_path: PathBuf,
    images: HashMap<PathBuf, Image>,
    themes: HashMap<PathBuf, PresentationTheme>,
    external_snippets: HashMap<PathBuf, String>,
    image_registry: ImageRegistry,
    watcher: FileWatcherHandle,
}

impl Resources {
    /// Construct a new resource manager over the provided based path.
    ///
    /// Any relative paths will be assumed to be relative to the given base.
    pub fn new<P: Into<PathBuf>>(base_path: P, image_registry: ImageRegistry) -> Self {
        let watcher = FileWatcher::spawn();
        Self {
            base_path: base_path.into(),
            images: Default::default(),
            themes: Default::default(),
            external_snippets: Default::default(),
            image_registry,
            watcher,
        }
    }

    pub(crate) fn watch_presentation_file(&self, path: PathBuf) {
        self.watcher.send(WatchEvent::WatchFile { path, watch_forever: true });
    }

    /// Get the image at the given path.
    pub(crate) fn image<P: AsRef<Path>>(&mut self, path: P) -> Result<Image, LoadImageError> {
        let path = self.base_path.join(path);
        if let Some(image) = self.images.get(&path) {
            return Ok(image.clone());
        }

        let image = self.image_registry.register_resource(path.clone())?;
        self.images.insert(path, image.clone());
        Ok(image)
    }

    /// Get the theme at the given path.
    pub(crate) fn theme<P: AsRef<Path>>(&mut self, path: P) -> Result<PresentationTheme, LoadThemeError> {
        let path = self.base_path.join(path);
        if let Some(theme) = self.themes.get(&path) {
            return Ok(theme.clone());
        }

        let theme = PresentationTheme::from_path(&path)?;
        self.themes.insert(path, theme.clone());
        Ok(theme)
    }

    /// Get the external snippet at the given path.
    pub(crate) fn external_snippet<P: AsRef<Path>>(&mut self, path: P) -> io::Result<String> {
        let path = self.base_path.join(path);
        if let Some(contents) = self.external_snippets.get(&path) {
            return Ok(contents.clone());
        }

        let contents = fs::read_to_string(&path)?;
        self.watcher.send(WatchEvent::WatchFile { path: path.clone(), watch_forever: false });
        self.external_snippets.insert(path, contents.clone());
        Ok(contents)
    }

    pub(crate) fn resources_modified(&mut self) -> bool {
        self.watcher.has_modifications()
    }

    pub(crate) fn clear_watches(&mut self) {
        self.watcher.send(WatchEvent::ClearWatches);
        // We could do better than this but this works for now.
        self.external_snippets.clear();
    }

    /// Clears all resources.
    pub(crate) fn clear(&mut self) {
        self.images.clear();
        self.themes.clear();
    }
}

/// An error loading an image.
#[derive(thiserror::Error, Debug)]
pub enum LoadImageError {
    #[error("io error reading {0}: {1}")]
    Io(PathBuf, io::Error),

    #[error(transparent)]
    RegisterImage(#[from] RegisterImageError),
}

/// Watches for file changes.
///
/// This uses polling rather than something fancier like `inotify`. The latter turned out to make
/// code too complex for little added gain. This instead keeps the last modified time for all
/// watched paths and uses that to determine if they've changed.
struct FileWatcher {
    receiver: Receiver<WatchEvent>,
    watches: HashMap<PathBuf, WatchMetadata>,
    modifications: Arc<AtomicBool>,
}

impl FileWatcher {
    fn spawn() -> FileWatcherHandle {
        let (sender, receiver) = channel();
        let modifications = Arc::new(AtomicBool::default());
        let handle = FileWatcherHandle { sender, modifications: modifications.clone() };
        thread::spawn(move || {
            let watcher = FileWatcher { receiver, watches: Default::default(), modifications };
            watcher.run();
        });
        handle
    }

    fn run(mut self) {
        loop {
            if let Ok(event) = self.receiver.try_recv() {
                self.handle_event(event);
            }
            if self.watches_modified() {
                self.modifications.store(true, Ordering::Relaxed);
            }
            thread::sleep(LOOP_INTERVAL);
        }
    }

    fn handle_event(&mut self, event: WatchEvent) {
        match event {
            WatchEvent::ClearWatches => {
                let new_watches =
                    mem::take(&mut self.watches).into_iter().filter(|(_, meta)| meta.watch_forever).collect();
                self.watches = new_watches;
            }
            WatchEvent::WatchFile { path, watch_forever } => {
                let last_modification =
                    fs::metadata(&path).and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH);
                let meta = WatchMetadata { last_modification, watch_forever };
                self.watches.insert(path, meta);
            }
        }
    }

    fn watches_modified(&mut self) -> bool {
        let mut modifications = false;
        for (path, meta) in &mut self.watches {
            let Ok(metadata) = fs::metadata(path) else {
                // If the file no longer exists, it's technically changed since last time.
                modifications = true;
                continue;
            };
            let Ok(modified_time) = metadata.modified() else {
                continue;
            };
            if modified_time > meta.last_modification {
                meta.last_modification = modified_time;
                modifications = true;
            }
        }
        modifications
    }
}

struct WatchMetadata {
    last_modification: SystemTime,
    watch_forever: bool,
}

struct FileWatcherHandle {
    sender: Sender<WatchEvent>,
    modifications: Arc<AtomicBool>,
}

impl FileWatcherHandle {
    fn send(&self, event: WatchEvent) {
        let _ = self.sender.send(event);
    }

    fn has_modifications(&mut self) -> bool {
        self.modifications.swap(false, Ordering::Relaxed)
    }
}

enum WatchEvent {
    /// Clear all watched files.
    ClearWatches,

    /// Add a file to the watch list.
    WatchFile { path: PathBuf, watch_forever: bool },
}
