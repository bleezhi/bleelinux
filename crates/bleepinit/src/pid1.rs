use super::*;
use anyhow::{Result, Context};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc, watch};
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct InitConfig {
    pub default_target: String,
    pub rescue_target: String,
    pub log_level: String,
    pub log_target: LogTarget,
    pub cgroups_v2: bool,
    pub unified_cgroup_hierarchy: bool,
    pub default_slice: String,
    pub systemd_compat: bool,
    pub openrc_compat: bool,
    pub runit_compat: bool,
    pub s6_compat: bool,
}

impl Default for InitConfig {
    fn default() -> Self {
        Self {
            default_target: "multi-user.target".into(),
            rescue_target: "rescue.target".into(),
            log_level: "info".into(),
            log_target: LogTarget::Journal,
            cgroups_v2: true,
            unified_cgroup_hierarchy: true,
            default_slice: "system.slice".into(),
            systemd_compat: true,
            openrc_compat: true,
            runit_compat: true,
            s6_compat: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogTarget {
    Console,
    Journal,
    Syslog,
    Kmsg,
    File(PathBuf),
}

pub struct Pid1 {
    config: InitConfig,
    supervisor: Arc<Mutex<Supervisor>>,
    service_manager: Arc<ServiceManager>,
    shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,
}

impl Pid1 {
    pub async fn new(config: InitConfig) -> Result<Self> {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        Ok(Self {
            config: config.clone(),
            supervisor: Arc::new(Mutex::new(Supervisor::new())),
            service_manager: Arc::new(ServiceManager::new(config)),
            shutdown_tx,
            shutdown_rx,
        })
    }

    pub async fn run(self) -> Result<()> {
        info!("bleepinit starting as PID 1");
        self.setup_mounts().await?;
        self.setup_cgroups().await?;
        self.start_essential_services().await?;
        self.service_manager.start_default_target(&self.supervisor).await?;
        self.main_loop().await?;
        self.shutdown().await?;
        Ok(())
    }

    async fn setup_mounts(&self) -> Result<()> {
        use nix::mount::{mount, MsFlags};
        use std::path::Path;
        let _ = mount::<str, str, str, str>(Some("proc"), "/proc", Some("proc"), MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV, None);
        let _ = mount::<str, str, str, str>(Some("sysfs"), "/sys", Some("sysfs"), MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV, None);
        let _ = mount::<str, str, str, str>(Some("devtmpfs"), "/dev", Some("devtmpfs"), MsFlags::MS_NOSUID, None);
        let _ = mount::<str, str, str, str>(Some("tmpfs"), "/run", Some("tmpfs"), MsFlags::MS_NOSUID | MsFlags::MS_NODEV, None);
        let _ = std::fs::create_dir_all("/run/lock");
        info!("essential filesystems mounted (best effort)");
        Ok(())
    }

    async fn setup_cgroups(&self) -> Result<()> {
        crate::cgroup::ensure_slices().ok();
        Ok(())
    }

    async fn start_essential_services(&self) -> Result<()> {
        info!("essential services: journald/udevd would start here");
        Ok(())
    }

    async fn main_loop(&self) -> Result<()> {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate())?;
        let mut sigint = signal(SignalKind::interrupt())?;
        let mut sigchld = signal(SignalKind::child())?;
        let mut sighup = signal(SignalKind::hangup())?;
        let mut rx = self.shutdown_tx.subscribe();
        loop {
            tokio::select! {
                _ = sigterm.recv() => { info!("SIGTERM, shutting down"); let _ = self.shutdown_tx.send(true); break; }
                _ = sigint.recv() => { info!("SIGINT, shutting down"); let _ = self.shutdown_tx.send(true); break; }
                _ = sighup.recv() => { info!("SIGHUP, reload"); let _ = self.service_manager.reload().await; }
                _ = sigchld.recv() => { self.supervisor.lock().await.reap_children().await; }
                _ = rx.changed() => { if *rx.borrow() { break; } }
            }
        }
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        info!("shutting down...");
        self.service_manager.stop_all().await?;
        self.supervisor.lock().await.stop_all().await?;
        let _ = std::process::Command::new("sync").status();
        info!("shutdown complete");
        Ok(())
    }
}

pub fn setup_pid1() -> Result<()> {
    let config = InitConfig::default();
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    rt.block_on(async {
        crate::logger::init_logging("info").ok();
        let pid1 = Pid1::new(config).await?;
        pid1.run().await
    })
}
