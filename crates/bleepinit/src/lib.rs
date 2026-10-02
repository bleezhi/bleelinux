pub mod pid1;
pub mod service;
pub mod supervisor;
pub mod compat;
pub mod socket;
pub mod cgroup;
pub mod mount;
pub mod logger;
pub mod config;

use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

pub use pid1::*;
pub use service::*;
pub use supervisor::*;
pub use compat::*;
pub use socket::*;
pub use cgroup::*;
pub use mount::*;
pub use logger::*;
pub use config::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServiceDefinition {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub exec: Vec<String>,
    pub exec_start_pre: Vec<Vec<String>>,
    pub exec_start_post: Vec<Vec<String>>,
    pub exec_stop: Vec<String>,
    pub exec_stop_post: Vec<Vec<String>>,
    pub exec_reload: Vec<String>,
    pub working_directory: Option<PathBuf>,
    pub user: Option<String>,
    pub group: Option<String>,
    pub environment: HashMap<String, String>,
    pub capabilities: Vec<String>,
    pub ambient_capabilities: Vec<String>,
    pub no_new_privileges: bool,
    pub private_tmp: bool,
    pub private_devices: bool,
    pub protect_system: bool,
    pub protect_home: bool,
    pub protect_kernel_tunables: bool,
    pub protect_control_groups: bool,
    pub restrict_address_families: Vec<String>,
    pub restrict_namespaces: bool,
    pub lock_personality: bool,
    pub memory_deny_write_execute: bool,
    pub system_call_filter: Vec<String>,
    pub system_call_archs: Vec<String>,
    pub nice: i32,
    pub io_priority: i32,
    pub cpu_shares: u64,
    pub memory_limit: Option<u64>,
    pub tasks_max: Option<u64>,
    pub dependencies: Vec<Dependency>,
    pub conflicts: Vec<String>,
    pub before: Vec<String>,
    pub after: Vec<String>,
    pub requires: Vec<String>,
    pub wanted_by: Vec<String>,
    pub required_by: Vec<String>,
    pub part_of: Vec<String>,
    pub binds_to: Vec<String>,
    pub reload_propagated_from: Vec<String>,
    pub on_failure: Vec<String>,
    pub start_limit_interval: u64,
    pub start_limit_burst: u32,
    pub restart: RestartPolicy,
    pub restart_sec: u64,
    pub timeout_start: u64,
    pub timeout_stop: u64,
    pub timeout_restart: u64,
    pub watchdog_sec: u64,
    pub type_: ServiceType,
    pub pid_file: Option<PathBuf>,
    pub notify_access: NotifyAccess,
    pub sockets: Vec<SocketSpec>,
    pub slice: Option<String>,
    pub delegate: bool,
    pub dynamic_user: bool,
    pub dynamic_group: bool,
    pub state_directory: Option<PathBuf>,
    pub cache_directory: Option<PathBuf>,
    pub logs_directory: Option<PathBuf>,
    pub configuration_directory: Option<PathBuf>,
    pub runtime_directory: Option<PathBuf>,
    pub temporary_file_system: Vec<TmpfsSpec>,
    pub mount_flags: MountFlags,
    pub read_write_paths: Vec<PathBuf>,
    pub read_only_paths: Vec<PathBuf>,
    pub inaccessible_paths: Vec<PathBuf>,
    pub exec_paths: Vec<PathBuf>,
    pub no_exec_paths: Vec<PathBuf>,
    pub se_linux_context: Option<String>,
    pub app_armor_profile: Option<String>,
    pub syslog_identifier: Option<String>,
    pub syslog_facility: Option<String>,
    pub syslog_level: Option<String>,
    pub syslog_level_prefix: bool,
    pub standard_input: StandardIO,
    pub standard_output: StandardIO,
    pub standard_error: StandardIO,
    pub tty_path: Option<PathBuf>,
    pub tty_reset: bool,
    pub ttyv_hangup: bool,
    pub tty_vt_disallocate: bool,
    pub pam_name: Option<String>,
    pub keyring_mode: KeyringMode,
    pub remove_ipc: bool,
    pub cpu_affinity: Vec<u32>,
    pub numa_policy: Option<String>,
    pub io_device_weight: Vec<(PathBuf, u64)>,
    pub io_read_bandwidth_max: Vec<(PathBuf, u64)>,
    pub io_write_bandwidth_max: Vec<(PathBuf, u64)>,
    pub device_policy: DevicePolicy,
    pub device_allow: Vec<String>,
    pub control_group_attributes: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceType {
    Simple,
    Forking,
    Oneshot,
    Dbus,
    Notify,
    Idle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RestartPolicy {
    No,
    OnSuccess,
    OnFailure,
    OnAbnormal,
    OnWatchdog,
    OnAbort,
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NotifyAccess {
    None,
    Main,
    Exec,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StandardIO {
    Null,
    Tty,
    TtyForce,
    Socket,
    Fd,
    File,
    Journal,
    Syslog,
    Kmsg,
    JournalAndConsole,
    SyslogAndConsole,
    KmsgAndConsole,
    Inherit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeyringMode {
    Inherit,
    Private,
    Shared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DevicePolicy {
    Auto,
    Closed,
    Strict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub type_: DependencyType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DependencyType {
    Requires,
    Requisite,
    Wants,
    BindsTo,
    PartOf,
    Conflicts,
    Before,
    After,
    OnFailure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocketSpec {
    pub name: String,
    pub listen: Vec<String>,
    pub socket_type: SocketType,
    pub protocol: Option<String>,
    pub backlog: Option<u32>,
    pub bind_ipv6_only: Option<bool>,
    pub bind_to_device: Option<String>,
    pub service: Option<String>,
    pub remove_on_stop: bool,
    pub symlinks: Vec<String>,
    pub file_descriptor_name: Option<String>,
    pub free_bind: bool,
    pub transparent: bool,
    pub broadcast: bool,
    pub multicast_loop: bool,
    pub keep_alive: bool,
    pub keep_alive_time: Option<u32>,
    pub keep_alive_interval: Option<u32>,
    pub keep_alive_probes: Option<u32>,
    pub no_delay: bool,
    pub priority: Option<i32>,
    pub receive_buffer: Option<u32>,
    pub send_buffer: Option<u32>,
    pub iptos: Option<u32>,
    pub ipttl: Option<u32>,
    pub mark: Option<u32>,
    pub reuse_port: bool,
    pub smack_label: Option<String>,
    pub smack_label_ipin: Option<String>,
    pub smack_label_ipout: Option<String>,
    pub se_linux_context: Option<String>,
    pub tcp_congestion: Option<String>,
    pub nodelay: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SocketType {
    Stream,
    Datagram,
    SeqPacket,
    Fifo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmpfsSpec {
    pub path: PathBuf,
    pub size: Option<String>,
    pub mode: Option<u32>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub noexec: bool,
    pub nosuid: bool,
    pub nodev: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountFlags {
    pub shared: bool,
    pub slave: bool,
    pub private: bool,
    pub unbindable: bool,
    pub recursive: bool,
}

impl Default for ServiceDefinition {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::new(),
            description: String::new(),
            exec: Vec::new(),
            exec_start_pre: Vec::new(),
            exec_start_post: Vec::new(),
            exec_stop: Vec::new(),
            exec_stop_post: Vec::new(),
            exec_reload: Vec::new(),
            working_directory: None,
            user: None,
            group: None,
            environment: HashMap::new(),
            capabilities: Vec::new(),
            ambient_capabilities: Vec::new(),
            no_new_privileges: true,
            private_tmp: true,
            private_devices: true,
            protect_system: true,
            protect_home: true,
            protect_kernel_tunables: true,
            protect_control_groups: true,
            restrict_address_families: vec!["AF_UNIX".into(), "AF_INET".into(), "AF_INET6".into()],
            restrict_namespaces: true,
            lock_personality: true,
            memory_deny_write_execute: true,
            system_call_filter: Vec::new(),
            system_call_archs: Vec::new(),
            nice: 0,
            io_priority: 0,
            cpu_shares: 1024,
            memory_limit: None,
            tasks_max: None,
            dependencies: Vec::new(),
            conflicts: Vec::new(),
            before: Vec::new(),
            after: Vec::new(),
            requires: Vec::new(),
            wanted_by: Vec::new(),
            required_by: Vec::new(),
            part_of: Vec::new(),
            binds_to: Vec::new(),
            reload_propagated_from: Vec::new(),
            on_failure: Vec::new(),
            start_limit_interval: 10,
            start_limit_burst: 5,
            restart: RestartPolicy::OnFailure,
            restart_sec: 1,
            timeout_start: 90,
            timeout_stop: 90,
            timeout_restart: 90,
            watchdog_sec: 0,
            type_: ServiceType::Simple,
            pid_file: None,
            notify_access: NotifyAccess::Main,
            sockets: Vec::new(),
            slice: None,
            delegate: false,
            dynamic_user: false,
            dynamic_group: false,
            state_directory: None,
            cache_directory: None,
            logs_directory: None,
            configuration_directory: None,
            runtime_directory: None,
            temporary_file_system: Vec::new(),
            mount_flags: MountFlags { shared: false, slave: false, private: true, unbindable: false, recursive: false },
            read_write_paths: Vec::new(),
            read_only_paths: Vec::new(),
            inaccessible_paths: Vec::new(),
            exec_paths: Vec::new(),
            no_exec_paths: Vec::new(),
            se_linux_context: None,
            app_armor_profile: None,
            syslog_identifier: None,
            syslog_facility: None,
            syslog_level: None,
            syslog_level_prefix: true,
            standard_input: StandardIO::Null,
            standard_output: StandardIO::Journal,
            standard_error: StandardIO::Inherit,
            tty_path: None,
            tty_reset: false,
            ttyv_hangup: false,
            tty_vt_disallocate: false,
            pam_name: None,
            keyring_mode: KeyringMode::Private,
            remove_ipc: false,
            cpu_affinity: Vec::new(),
            numa_policy: None,
            io_device_weight: Vec::new(),
            io_read_bandwidth_max: Vec::new(),
            io_write_bandwidth_max: Vec::new(),
            device_policy: DevicePolicy::Auto,
            device_allow: Vec::new(),
            control_group_attributes: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceState {
    Inactive,
    Activating,
    Active,
    Deactivating,
    Failed,
    Maintenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub name: String,
    pub state: ServiceState,
    pub sub_state: String,
    pub pid: Option<u32>,
    pub main_pid: Option<u32>,
    pub started_at: Option<DateTime<Utc>>,
    pub exited_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub exit_signal: Option<i32>,
    pub memory_current: Option<u64>,
    pub cpu_usage: Option<u64>,
    pub tasks_current: Option<u64>,
    pub delegated: bool,
    pub slice: Option<String>,
    pub control_group: Option<String>,
    pub description: String,
    pub load_state: LoadState,
    pub active_state: ActiveState,
    pub sub_state_str: String,
    pub fragment_path: Option<PathBuf>,
    pub unit_file_state: UnitFileState,
    pub unit_file_preset: UnitFilePreset,
    pub transient: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LoadState {
    Loaded,
    NotFound,
    BadSetting,
    Masked,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActiveState {
    Active,
    Reloading,
    Inactive,
    Failed,
    Activating,
    Deactivating,
    Maintenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnitFileState {
    Enabled,
    EnabledRuntime,
    Disabled,
    Static,
    Masked,
    MaskedRuntime,
    Alias,
    Generated,
    Transient,
    Bad,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnitFilePreset {
    Enabled,
    Disabled,
    Unknown,
}