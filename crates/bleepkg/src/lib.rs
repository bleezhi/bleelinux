use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use semver::Version;
use uuid::Uuid;
use chrono::{DateTime, Utc};

pub mod archive;
pub mod metadata;
pub mod install;
pub mod verify;

pub use archive::*;
pub use metadata::*;
pub use install::*;
pub use verify::*;

#[derive(Debug, Error)]
pub enum BleepkgError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("TOML serialize error: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("DB error: {0}")]
    Db(String),
    #[error("Archive error: {0}")]
    Archive(String),
    #[error("Verification failed: {0}")]
    Verify(String),
    #[error("Invalid package: {0}")]
    Invalid(String),
    #[error("Dependency error: {0}")]
    Dependency(String),
    #[error("System time error: {0}")]
    Time(#[from] std::time::SystemTimeError),
    #[error("{0}")]
    Any(String),
}

impl From<rusqlite::Error> for BleepkgError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Db(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, BleepkgError>;