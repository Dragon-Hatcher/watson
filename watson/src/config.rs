use serde::Deserialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

const CONFIG_FILE_NAME: &str = "watson.toml";

/// Search for watson.toml starting from the given directory and moving up the directory tree
pub fn find_config_file() -> Result<PathBuf, ConfigError> {
    let current_dir =
        env::current_dir().map_err(|_| ConfigError::IoError)?;

    let start_dir = current_dir
        .canonicalize()
        .map_err(|_| ConfigError::IoError)?;

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
    build_dir: PathBuf,
    math_dir: PathBuf,
    lua_dir: PathBuf,
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
        let math_dir = project_dir.join("math");
        let lua_dir = project_dir.join("script");

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
            build_dir,
            math_dir,
            lua_dir,
            book,
        })
    }

    pub fn math_dir(&self) -> &Path {
        &self.math_dir
    }

    pub fn lua_dir(&self) -> &Path {
        &self.lua_dir
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
            fs::read_to_string(path).map_err(|_| ConfigError::IoError)?;

        toml::from_str(&contents).map_err(|_| ConfigError::ParseError)
    }
}

#[derive(Debug)]
pub enum ConfigError {
    NotFound,
    IoError,
    ParseError,
}
