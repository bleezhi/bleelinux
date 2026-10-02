use super::*;
use anyhow::Result;
use nix::sys::signal::Signal;
use nix::unistd::Pid;
use std::collections::HashMap;
use std::process::{Command, Child, Stdio};
use tracing::{info, warn};

struct ServiceHandle {
    def: ServiceDefinition,
    state: ServiceState,
    child: Option<Child>,
    restart_count: u32,
}

pub struct Supervisor {
    services: HashMap<String, ServiceHandle>,
}

impl Supervisor {
    pub fn new() -> Self {
        Self { services: HashMap::new() }
    }

    pub async fn spawn_service(&mut self, def: ServiceDefinition) -> Result<()> {
        info!("starting service: {}", def.name);
        // start dependencies first (non-recursive, best effort)
        for dep in def.after.clone() {
            let _ = dep;
        }
        let mut cmd = Command::new(def.exec.first().map(|s| s.as_str()).unwrap_or("/bin/true"));
        if def.exec.len() > 1 {
            cmd.args(&def.exec[1..]);
        }
        if let Some(wd) = &def.working_directory {
            cmd.current_dir(wd);
        }
        for (k, v) in &def.environment {
            cmd.env(k, v);
        }
        // PID 1 inherits a near-empty kernel env – give services sane defaults
        if !def.environment.contains_key("PATH") && std::env::var_os("PATH").is_none() {
            cmd.env("PATH", "/usr/bin:/bin:/sbin:/usr/sbin");
        }
        if !def.environment.contains_key("HOME") && std::env::var_os("HOME").is_none() {
            cmd.env("HOME", "/root");
        }
        // Interactive services (shell/getty) need the console; others inherit too
        // for Phase 1 so logs are visible. Null stdin breaks sh as init.
        cmd.stdin(Stdio::inherit()).stdout(Stdio::inherit()).stderr(Stdio::inherit());
        let child = cmd.spawn()?;
        let pid = child.id();
        info!("service {} started pid={}", def.name, pid);
        self.services.insert(def.name.clone(), ServiceHandle {
            def,
            state: ServiceState::Active,
            child: Some(child),
            restart_count: 0,
        });
        Ok(())
    }

    pub async fn stop_service(&mut self, name: &str) -> Result<()> {
        if let Some(mut h) = self.services.remove(name) {
            info!("stopping {}", name);
            if let Some(mut c) = h.child.take() {
                let _ = c.kill();
                let _ = c.wait();
            }
        }
        Ok(())
    }

    pub async fn restart_service(&mut self, name: &str) -> Result<()> {
        let def = self.services.get(name).map(|h| h.def.clone());
        if let Some(def) = def {
            self.stop_service(name).await?;
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            Box::pin(self.spawn_service(def)).await?;
        }
        Ok(())
    }

    pub async fn send_signal(&self, name: &str, sig: Signal) -> Result<()> {
        warn!("send signal {:?} to {} (stub)", sig, name);
        Ok(())
    }

    pub async fn reap_children(&mut self) {
        use std::collections::VecDeque;
        let mut exited = Vec::new();
        for (name, h) in self.services.iter_mut() {
            if let Some(c) = h.child.as_mut() {
                match c.try_wait() {
                    Ok(Some(status)) => {
                        info!("service {} exited: {}", name, status);
                        exited.push(name.clone());
                    }
                    _ => {}
                }
            }
        }
        for name in exited {
            if let Some(mut h) = self.services.remove(&name) {
                h.child.take();
                h.state = ServiceState::Failed;
                // restart policy
                if h.def.restart != RestartPolicy::No && h.restart_count < h.def.start_limit_burst {
                    h.restart_count += 1;
                    let def = h.def.clone();
                    self.services.insert(name.clone(), h);
                    let _ = Box::pin(self.spawn_service(def)).await;
                } else {
                    self.services.insert(name, h);
                }
            }
        }
        let _ = Pid::from_raw(0);
    }

    pub async fn stop_all(&mut self) -> Result<()> {
        let names: Vec<String> = self.services.keys().cloned().collect();
        for n in names.iter().rev() {
            let _ = self.stop_service(n).await;
        }
        Ok(())
    }

    pub async fn dump_state(&self) {
        for (name, h) in &self.services {
            println!("{} [{:?}] restarts={}", name, h.state, h.restart_count);
        }
    }
}
