use super::raw::PresentationTheme;
use std::{collections::BTreeMap, fs, io, path::Path};

include!(concat!(env!("OUT_DIR"), "/themes.rs"));

#[derive(Default)]
pub struct PresentationThemeRegistry {
    custom_themes: BTreeMap<String, PresentationTheme>,
}

impl PresentationThemeRegistry {
    /// Loads a theme from its name.
    pub fn load_by_name(&self, name: &str) -> Option<PresentationTheme> {
        match THEMES.get(name) {
            Some(contents) => {
                // This is going to be caught by the test down here.
                let theme = serde_yaml::from_slice(contents).expect("corrupted theme");
                Some(theme)
            }
            None => self.custom_themes.get(name).cloned(),
        }
    }

    /// Register all the themes in the given directory.
    pub fn register_from_directory<P: AsRef<Path>>(&mut self, path: P) -> Result<(), LoadThemeError> {
        let handle = match fs::read_dir(&path) {
            Ok(handle) => handle,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e.into()),
        };
        let mut dependencies = BTreeMap::new();
        for entry in handle {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let Some(file_name) = entry.file_name().to_str().map(ToOwned::to_owned) else {
                continue;
            };
            if metadata.is_file() && file_name.ends_with(".yaml") {
                let theme_name = file_name.trim_end_matches(".yaml");
                if THEMES.contains_key(theme_name) {
                    return Err(LoadThemeError::Duplicate(theme_name.into()));
                }
                let theme = PresentationTheme::from_path(entry.path())?;
                let base = theme.extends.clone();
                self.custom_themes.insert(theme_name.into(), theme);
                dependencies.insert(theme_name.to_string(), base);
            }
        }
        let mut graph = ThemeGraph::new(dependencies);
        for theme_name in graph.dependents.keys() {
            let theme_name = theme_name.as_str();
            if !THEMES.contains_key(theme_name) && !self.custom_themes.contains_key(theme_name) {
                return Err(LoadThemeError::ExtendedThemeNotFound(theme_name.into()));
            }
        }

        while let Some(theme_name) = graph.pop() {
            self.extend_theme(&theme_name)?;
        }
        if !graph.dependents.is_empty() {
            return Err(LoadThemeError::ExtensionLoop(graph.dependents.into_keys().collect()));
        }
        Ok(())
    }

    fn extend_theme(&mut self, theme_name: &str) -> Result<(), LoadThemeError> {
        let Some(base_name) = self.custom_themes.get(theme_name).expect("theme not found").extends.clone() else {
            return Ok(());
        };
        let Some(base_theme) = self.load_by_name(&base_name) else {
            return Err(LoadThemeError::ExtendedThemeNotFound(base_name.clone()));
        };
        let theme = self.custom_themes.get_mut(theme_name).expect("theme not found");
        *theme = merge_struct::merge(&base_theme, theme)
            .map_err(|e| LoadThemeError::Corrupted(base_name.to_string(), e.into()))?;
        Ok(())
    }

    /// Get all the registered theme names.
    pub fn theme_names(&self) -> Vec<String> {
        let builtin_themes = THEMES.keys().map(|name| name.to_string());
        let themes = self.custom_themes.keys().cloned().chain(builtin_themes).collect();
        themes
    }
}

struct ThemeGraph {
    dependents: BTreeMap<String, Vec<String>>,
    ready: Vec<String>,
}

impl ThemeGraph {
    fn new<I>(dependencies: I) -> Self
    where
        I: IntoIterator<Item = (String, Option<String>)>,
    {
        let mut dependents: BTreeMap<_, Vec<_>> = BTreeMap::new();
        let mut ready = Vec::new();
        for (name, extends) in dependencies {
            dependents.entry(name.clone()).or_default();
            match extends {
                // If we extend from a non built in theme, make ourselves their dependent
                Some(base) if !THEMES.contains_key(base.as_str()) => {
                    dependents.entry(base).or_default().push(name);
                }
                // Otherwise this theme is ready to be processed
                _ => ready.push(name),
            }
        }
        Self { dependents, ready }
    }

    fn pop(&mut self) -> Option<String> {
        let theme = self.ready.pop()?;
        if let Some(dependents) = self.dependents.remove(&theme) {
            self.ready.extend(dependents);
        }
        Some(theme)
    }
}

/// An error loading a presentation theme.
#[derive(thiserror::Error, Debug)]
pub enum LoadThemeError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("theme '{0}' is corrupted: {1}")]
    Corrupted(String, Box<dyn std::error::Error>),

    #[error("duplicate custom theme '{0}'")]
    Duplicate(String),

    #[error("extended theme does not exist: {0}")]
    ExtendedThemeNotFound(String),

    #[error("theme has an extension loop involving: {0:?}")]
    ExtensionLoop(Vec<String>),
}

#[cfg(test)]
mod test {
    use crate::resource::Resources;

    use super::*;
    use tempfile::{TempDir, tempdir};

    fn write_theme(name: &str, theme: PresentationTheme, directory: &TempDir) {
        let theme = serde_yaml::to_string(&theme).unwrap();
        let file_name = format!("{name}.yaml");
        fs::write(directory.path().join(file_name), theme).expect("writing theme");
    }

    #[test]
    fn validate_themes() {
        let themes = PresentationThemeRegistry::default();
        for theme_name in THEMES.keys() {
            let Some(theme) = themes.load_by_name(theme_name).clone() else {
                panic!("theme '{theme_name}' is corrupted");
            };

            // Built-in themes can't use this because... I don't feel like supporting this now.
            assert!(theme.extends.is_none(), "theme '{theme_name}' uses extends");

            let merged = merge_struct::merge(&PresentationTheme::default(), &theme);
            assert!(merged.is_ok(), "theme '{theme_name}' can't be merged: {}", merged.unwrap_err());

            let resources = Resources::new("/tmp/foo", "/tmp/foo", Default::default());
            crate::theme::PresentationTheme::new(&theme, &resources).expect("malformed theme");
        }
    }

    #[test]
    fn load_custom() {
        let directory = tempdir().expect("creating tempdir");
        write_theme(
            "potato",
            PresentationTheme { extends: Some("dark".to_string()), ..Default::default() },
            &directory,
        );

        let mut themes = PresentationThemeRegistry::default();
        themes.register_from_directory(directory.path()).expect("loading themes");
        let mut theme = themes.load_by_name("potato").expect("theme not found");

        // Since we extend the dark theme they must match after we remove the "extends" field.
        let dark = themes.load_by_name("dark");
        theme.extends.take().expect("no extends");
        assert_eq!(serde_yaml::to_string(&theme).unwrap(), serde_yaml::to_string(&dark).unwrap());
    }

    #[test]
    fn load_derive_chain() {
        let directory = tempdir().expect("creating tempdir");
        write_theme("A", PresentationTheme { extends: Some("dark".to_string()), ..Default::default() }, &directory);
        write_theme("B", PresentationTheme { extends: Some("C".to_string()), ..Default::default() }, &directory);
        write_theme("C", PresentationTheme { extends: Some("A".to_string()), ..Default::default() }, &directory);
        write_theme("D", PresentationTheme::default(), &directory);

        let mut themes = PresentationThemeRegistry::default();
        themes.register_from_directory(directory.path()).expect("loading themes");
        themes.load_by_name("A").expect("A not found");
        themes.load_by_name("B").expect("B not found");
        themes.load_by_name("C").expect("C not found");
        themes.load_by_name("D").expect("D not found");
    }

    #[test]
    fn invalid_derives() {
        let directory = tempdir().expect("creating tempdir");
        write_theme(
            "A",
            PresentationTheme { extends: Some("non-existent-theme".to_string()), ..Default::default() },
            &directory,
        );

        let mut themes = PresentationThemeRegistry::default();
        themes.register_from_directory(directory.path()).expect_err("loading themes succeeded");
    }

    #[test]
    fn load_derive_chain_loop() {
        let directory = tempdir().expect("creating tempdir");
        write_theme("A", PresentationTheme { extends: Some("B".to_string()), ..Default::default() }, &directory);
        write_theme("B", PresentationTheme { extends: Some("A".to_string()), ..Default::default() }, &directory);

        let mut themes = PresentationThemeRegistry::default();
        let err = themes.register_from_directory(directory.path()).expect_err("loading themes succeeded");
        let LoadThemeError::ExtensionLoop(names) = err else { panic!("not an extension loop error") };
        assert_eq!(names, &["A", "B"]);
    }

    #[test]
    fn register_from_missing_directory() {
        let mut themes = PresentationThemeRegistry::default();
        let result = themes.register_from_directory("/tmp/presenterm/8ee2027983915ec78acc45027d874316");
        result.expect("loading failed");
    }
}
