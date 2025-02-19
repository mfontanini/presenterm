use crate::{
    terminal::image::{
        Image,
        printer::{ImageRegistry, RegisterImageError},
    },
    theme::{PresentationTheme, registry::LoadThemeError},
};
use std::{
    cell::RefCell,
    collections::HashMap,
    fs, io, mem,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender, channel},
    },
    thread,
    time::{Duration, SystemTime},
};

const LOOP_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Debug)]
struct ResourcesInner {
    images: HashMap<PathBuf, Image>,
    theme_images: HashMap<PathBuf, Image>,
    themes: HashMap<PathBuf, PresentationTheme>,
    external_snippets: HashMap<PathBuf, String>,
    base_path: PathBuf,
    themes_path: PathBuf,
    image_registry: ImageRegistry,
    watcher: FileWatcherHandle,
}

/// Manages resources pulled from the filesystem such as images.
///
/// All resources are cached so once a specific resource is loaded, looking it up with the same
/// path will involve an in-memory lookup.
#[derive(Clone, Debug)]
pub struct Resources {
    inner: Rc<RefCell<ResourcesInner>>,
}

impl Resources {
    /// Construct a new resource manager over the provided based path.
    ///
    /// Any relative paths will be assumed to be relative to the given base.
    pub fn new<P1, P2>(base_path: P1, themes_path: P2, image_registry: ImageRegistry) -> Self
    where
        P1: Into<PathBuf>,
        P2: Into<PathBuf>,
    {
        let watcher = FileWatcher::spawn();
        let inner = ResourcesInner {
            base_path: base_path.into(),
            themes_path: themes_path.into(),
            images: Default::default(),
            theme_images: Default::default(),
            themes: Default::default(),
            external_snippets: Default::default(),
            image_registry,
            watcher,
        };
        Self { inner: Rc::new(RefCell::new(inner)) }
    }

    pub(crate) fn watch_presentation_file(&self, path: PathBuf) {
        let inner = self.inner.borrow();
        inner.watcher.send(WatchEvent::WatchFile { path, watch_forever: true });
    }

    /// Get the image at the given path.
    pub(crate) fn image<P: AsRef<Path>>(&self, path: P) -> Result<Image, RegisterImageError> {
        let mut inner = self.inner.borrow_mut();
        let path = inner.base_path.join(path);
        if let Some(image) = inner.images.get(&path) {
            return Ok(image.clone());
        }

        let image = inner.image_registry.register_resource(path.clone())?;
        inner.images.insert(path, image.clone());
        Ok(image)
    }

    pub(crate) fn theme_image<P: AsRef<Path>>(&self, path: P) -> Result<Image, RegisterImageError> {
        match self.image(&path) {
            Ok(image) => return Ok(image),
            Err(RegisterImageError::Io(e)) if e.kind() != io::ErrorKind::NotFound => return Err(e.into()),
            _ => (),
        };

        let mut inner = self.inner.borrow_mut();
        let path = inner.themes_path.join(path);
        if let Some(image) = inner.theme_images.get(&path) {
            return Ok(image.clone());
        }

        let image = inner.image_registry.register_resource(path.clone())?;
        inner.theme_images.insert(path, image.clone());
        Ok(image)
    }

    /// Get the theme at the given path.
    pub(crate) fn theme<P: AsRef<Path>>(&self, path: P) -> Result<PresentationTheme, LoadThemeError> {
        let mut inner = self.inner.borrow_mut();
        let path = inner.base_path.join(path);
        if let Some(theme) = inner.themes.get(&path) {
            return Ok(theme.clone());
        }

        let theme = PresentationTheme::from_path(&path)?;
        inner.themes.insert(path, theme.clone());
        Ok(theme)
    }

    /// Get the external snippet at the given path.
    pub(crate) fn external_snippet<P: AsRef<Path>>(&self, path: P) -> io::Result<String> {
        let mut inner = self.inner.borrow_mut();
        let path = inner.base_path.join(path);
        if let Some(contents) = inner.external_snippets.get(&path) {
            return Ok(contents.clone());
        }

        let contents = fs::read_to_string(&path)?;
        inner.watcher.send(WatchEvent::WatchFile { path: path.clone(), watch_forever: false });
        inner.external_snippets.insert(path, contents.clone());
        Ok(contents)
    }

    pub(crate) fn resources_modified(&self) -> bool {
        let mut inner = self.inner.borrow_mut();
        inner.watcher.has_modifications()
    }

    pub(crate) fn clear_watches(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.watcher.send(WatchEvent::ClearWatches);
        // We could do better than this but this works for now.
        inner.external_snippets.clear();
    }

    /// Clears all resources.
    pub(crate) fn clear(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.images.clear();
        inner.themes.clear();
    }
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

#[derive(Debug)]
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
