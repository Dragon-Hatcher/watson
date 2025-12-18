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

#[derive(Debug, Clone)]
pub struct WatsonConfig {
    project_dir: PathBuf,
    build_dir: PathBuf,
    book_title: Option<String>,
}

impl WatsonConfig {
    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let config_file = WatsonConfigFile::from_file(path)?;
        let project_dir = path.parent().unwrap().canonicalize().unwrap();
        let build_dir = project_dir.join("build");

        Ok(Self {
            project_dir,
            build_dir,
            book_title: config_file.book.and_then(|b| b.title),
        })
    }

    pub fn project_dir(&self) -> &Path {
        &self.project_dir
    }

    pub fn build_dir(&self) -> &Path {
        &self.build_dir
    }

    pub fn book_title(&self) -> Option<&str> {
        self.book_title.as_deref()
    }
}

#[derive(Debug, Deserialize)]
struct WatsonConfigFile {
    book: Option<BookConfig>,
}

#[derive(Debug, Deserialize)]
struct BookConfig {
    title: Option<String>,
}

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
