use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::Result;
use bleepkg::{Architecture, Libc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub root_dir: PathBuf,
    pub db_path: PathBuf,
    pub cache_dir: PathBuf,
    pub build_dir: PathBuf,
    pub recipes_dir: PathBuf,
    pub hooks_dir: PathBuf,
    pub repos: Vec<RepoConfig>,
    pub target_arch: Architecture,
    pub target_libc: Libc,
    pub target_triplet: String,
    pub parallel_jobs: usize,
    pub verify_signatures: bool,
    pub trusted_keys: Vec<String>,
    pub colors: bool,
    pub progress_bars: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConfig {
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub arch: Architecture,
    pub libc: Libc,
    pub priority: i32,
    pub verify_signatures: bool,
    pub public_key: Option<String>,
    pub recipes_url: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        let root = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")).join(".bleelinux");
        Self {
            root_dir: root.clone(),
            db_path: root.join("db/packages.sqlite"),
            cache_dir: root.join("cache"),
            build_dir: root.join("build"),
            recipes_dir: root.join("recipes"),
            hooks_dir: root.join("hooks"),
            repos: vec![RepoConfig {
                name: "core".into(),
                url: "https://packages.bleelinux.org/core".into(),
                enabled: true,
                arch: Architecture::X86_64,
                libc: Libc::Glibc,
                priority: 10,
                verify_signatures: true,
                public_key: None,
                recipes_url: None,
            }],
            target_arch: Architecture::X86_64,
            target_libc: Libc::Glibc,
            target_triplet: "x86_64-unknown-linux-gnu".into(),
            parallel_jobs: num_cpus::get(),
            verify_signatures: true,
            trusted_keys: vec![],
            colors: true,
            progress_bars: true,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        // user config first, then system config, then defaults
        if let Some(dir) = dirs::config_dir() {
            let p = dir.join("bleelinux/config.toml");
            if p.exists() {
                return Ok(toml::from_str(&std::fs::read_to_string(p)?)?);
            }
        }
        let sys = PathBuf::from("/etc/bleelinux/config.toml");
        if sys.exists() {
            return Ok(toml::from_str(&std::fs::read_to_string(sys)?)?);
        }
        Ok(Self::default())
    }

    pub fn load_from(path: PathBuf) -> Result<Self> {
        if path.exists() {
            Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("/etc")).join("bleelinux/config.toml");
        self.save_to(&p)
    }

    pub fn save_to(&self, path: &std::path::Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }
}
