use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BleepInitConfigFile {
    pub default_target: Option<String>,
    pub log_level: Option<String>,
    pub systemd_compat: Option<bool>,
    pub openrc_compat: Option<bool>,
    pub runit_compat: Option<bool>,
    pub s6_compat: Option<bool>,
}

impl BleepInitConfigFile {
    pub fn load(path: &str) -> Result<super::InitConfig> {
        let mut cfg = super::InitConfig::default();
        if std::path::Path::new(path).exists() {
            let content = std::fs::read_to_string(path)?;
            let f: BleepInitConfigFile = toml::from_str(&content)?;
            if let Some(v) = f.default_target { cfg.default_target = v; }
            if let Some(v) = f.log_level { cfg.log_level = v; }
            if let Some(v) = f.systemd_compat { cfg.systemd_compat = v; }
            if let Some(v) = f.openrc_compat { cfg.openrc_compat = v; }
            if let Some(v) = f.runit_compat { cfg.runit_compat = v; }
            if let Some(v) = f.s6_compat { cfg.s6_compat = v; }
        }
        Ok(cfg)
    }
}

pub fn default_config_path() -> PathBuf {
    PathBuf::from("/etc/bleepinit/init.toml")
}
