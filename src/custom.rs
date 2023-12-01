use serde::Deserialize;
use std::{fs, io, path::Path};

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub defaults: DefaultsConfig,
}

impl Config {
    /// Load the config from a path.
    pub fn load(path: &Path) -> Result<Self, ConfigLoadError> {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e.into()),
        };
        let config = serde_yaml::from_str(&contents)?;
        Ok(config)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigLoadError {
    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("invalid configuration: {0}")]
    Invalid(#[from] serde_yaml::Error),
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct DefaultsConfig {
    pub theme: Option<String>,
}
