use super::*;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

pub struct ServiceManager {
    config: InitConfig,
    compat: CompatLayer,
    states: Arc<RwLock<HashMap<String, ServiceStatus>>>,
}

impl ServiceManager {
    pub fn new(config: InitConfig) -> Self {
        let compat = CompatLayer::new(config.clone());
        Self { config, compat, states: Arc::new(RwLock::new(HashMap::new())) }
    }

    pub async fn start_default_target(&self, supervisor: &Arc<Mutex<Supervisor>>) -> Result<()> {
        tracing::info!("starting default target {}", self.config.default_target);
        for dir in ["/etc/bleepinit", "/usr/lib/bleepinit"] {
            let entries = match std::fs::read_dir(dir) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().map(|x| x=="toml").unwrap_or(false) {
                    let name = match p.file_stem().and_then(|s| s.to_str()) {
                        Some(n) => n,
                        None => continue,
                    };
                    if name == "init" { continue; }
                    let def = match self.compat.load_service(name) {
                        Ok(d) => d,
                        Err(e) => {
                            tracing::warn!("skip {}: {}", name, e);
                            continue;
                        }
                    };
                    if def.wanted_by.iter().any(|t| *t==self.config.default_target) {
                        tracing::info!("starting {}", def.name);
                        if let Err(e) = supervisor.lock().await.spawn_service(def.clone()).await {
                            tracing::warn!("failed to start {}: {}", def.name, e);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn stop_all(&self) -> Result<()> {
        tracing::info!("stopping all services");
        Ok(())
    }

    pub async fn reload(&self) -> Result<()> {
        tracing::info!("reloading service manager");
        Ok(())
    }

    pub async fn status(&self, name: &str) -> Result<ServiceStatus> {
        let states = self.states.read().await;
        states.get(name).cloned().ok_or_else(|| anyhow::anyhow!("no such service {}", name))
    }
}
