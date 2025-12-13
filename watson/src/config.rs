use serde::Deserialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

const CONFIG_FILE_NAME: &str = "watson.toml";

/// Search for watson.toml starting from the given directory and moving up the directory tree
pub fn find_config_file() -> Result<PathBuf, ConfigError> {
    let current_dir =
        env::current_dir().map_err(|e| ConfigError::IoError(PathBuf::from("."), e))?;

    let start_dir = current_dir
        .canonicalize()
        .map_err(|e| ConfigError::IoError(current_dir.to_path_buf(), e))?;

    let mut current = start_dir.as_path();

    loop {
        let candidate = current.join(CONFIG_FILE_NAME);
        if candidate.exists() {
            return Ok(candidate);
        }

        current = match current.parent() {
            Some(parent) => parent,
            None => return Err(ConfigError::NotFound),
        };
    }
}

pub struct WatsonConfig {
    project_dir: PathBuf,
}

impl WatsonConfig {
    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let _ = WatsonConfigFile::from_file(path)?;
        let project_dir = path.parent().unwrap().canonicalize().unwrap();

        Ok(Self { project_dir })
    }

    pub fn project_dir(&self) -> &Path {
        &self.project_dir
    }
}

#[derive(Debug, Deserialize)]
struct WatsonConfigFile {}

impl WatsonConfigFile {
    /// Parse a watson.toml config file from the given path
    fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let contents =
            fs::read_to_string(path).map_err(|e| ConfigError::IoError(path.to_path_buf(), e))?;

        toml::from_str(&contents).map_err(|e| ConfigError::ParseError(path.to_path_buf(), e))
    }
}

#[derive(Debug)]
pub enum ConfigError {
    NotFound,
    IoError(PathBuf, std::io::Error),
    ParseError(PathBuf, toml::de::Error),
    InvalidPath(PathBuf),
}
