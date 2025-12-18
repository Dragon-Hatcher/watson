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
    src_dir: PathBuf,
    book: BookConfig,
}

#[derive(Debug, Clone)]
pub struct BookConfig {
    title: Option<String>,
    port: u16,
}

impl WatsonConfig {
    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let config_file = WatsonConfigFile::from_file(path)?;
        let project_dir = path.parent().unwrap().canonicalize().unwrap();
        let build_dir = project_dir.join("build");
        let src_dir = project_dir.join("src");

        let book = match config_file.book {
            Some(book_config) => BookConfig {
                title: book_config.title,
                port: book_config.port.unwrap_or(4747),
            },
            None => BookConfig {
                title: None,
                port: 4747,
            },
        };

        Ok(Self {
            project_dir,
            build_dir,
            src_dir,
            book,
        })
    }

    pub fn project_dir(&self) -> &Path {
        &self.project_dir
    }

    pub fn src_dir(&self) -> &Path {
        &self.src_dir
    }

    pub fn build_dir(&self) -> &Path {
        &self.build_dir
    }

    pub fn book(&self) -> &BookConfig {
        &self.book
    }
}

impl BookConfig {
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

#[derive(Debug, Deserialize)]
struct WatsonConfigFile {
    book: Option<BookConfigFile>,
}

#[derive(Debug, Deserialize)]
struct BookConfigFile {
    title: Option<String>,
    port: Option<u16>,
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
