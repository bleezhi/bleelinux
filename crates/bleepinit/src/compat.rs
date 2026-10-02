use super::*;
use anyhow::{Result, Context};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;

pub struct CompatLayer {
    config: InitConfig,
}

impl CompatLayer {
    pub fn new(config: InitConfig) -> Self {
        Self { config }
    }

    pub fn load_service(&self, name: &str) -> Result<ServiceDefinition> {
        if let Ok(def) = self.load_native(name) {
            return Ok(def);
        }
        if self.config.systemd_compat {
            if let Ok(def) = self.load_systemd(name) {
                return Ok(def);
            }
        }
        if self.config.openrc_compat {
            if let Ok(def) = self.load_openrc(name) {
                return Ok(def);
            }
        }
        if self.config.runit_compat {
            if let Ok(def) = self.load_runit(name) {
                return Ok(def);
            }
        }
        if self.config.s6_compat {
            if let Ok(def) = self.load_s6(name) {
                return Ok(def);
            }
        }
        Err(anyhow::anyhow!("Service not found: {}", name))
    }

    fn load_native(&self, name: &str) -> Result<ServiceDefinition> {
        let paths = [
            format!("/etc/bleepinit/{}.toml", name),
            format!("/usr/lib/bleepinit/{}.toml", name),
        ];
        for p in &paths {
            if Path::new(p).exists() {
                let content = fs::read_to_string(p)?;
                return Ok(toml::from_str(&content)?);
            }
        }
        Err(anyhow::anyhow!("native not found"))
    }

    pub fn load_systemd(&self, name: &str) -> Result<ServiceDefinition> {
        let unit_name = if name.ends_with(".service") { name.to_string() } else { format!("{}.service", name) };
        let paths = [
            format!("/etc/systemd/system/{}", unit_name),
            format!("/usr/lib/systemd/system/{}", unit_name),
            format!("/lib/systemd/system/{}", unit_name),
        ];
        for p in &paths {
            if Path::new(p).exists() {
                let content = fs::read_to_string(p)?;
                return self.parse_systemd_unit(&content, name);
            }
        }
        Err(anyhow::anyhow!("systemd unit not found: {}", name))
    }

    fn parse_systemd_unit(&self, content: &str, name: &str) -> Result<ServiceDefinition> {
        let mut def = ServiceDefinition::default();
        def.name = name.trim_end_matches(".service").to_string();

        let mut section = String::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                section = line[1..line.len()-1].to_lowercase();
                continue;
            }
            let (k, v) = match line.split_once('=') {
                Some(x) => (x.0.trim(), x.1.trim()),
                None => continue,
            };
            match (section.as_str(), k) {
                ("unit", "Description") => def.description = v.to_string(),
                ("unit", "After") => def.after.extend(split_list(v)),
                ("unit", "Before") => def.before.extend(split_list(v)),
                ("unit", "Requires") => def.requires.extend(split_list(v)),
                ("unit", "Wants") => {
                    for d in split_list(v) {
                        def.dependencies.push(Dependency { name: d.trim_end_matches(".service").to_string(), type_: DependencyType::Wants });
                    }
                }
                ("unit", "Conflicts") => def.conflicts.extend(split_list(v).iter().map(|s| s.trim_end_matches(".service").to_string())),
                ("service", "Type") => {
                    def.type_ = match v.to_lowercase().as_str() {
                        "forking" => ServiceType::Forking,
                        "oneshot" => ServiceType::Oneshot,
                        "dbus" => ServiceType::Dbus,
                        "notify" => ServiceType::Notify,
                        "idle" => ServiceType::Idle,
                        _ => ServiceType::Simple,
                    };
                }
                ("service", "ExecStart") => {
                    let cmd = strip_prefix_chars(v, &['-', ':', '+', '!']);
                    def.exec = shell_split(cmd);
                }
                ("service", "ExecStartPre") => def.exec_start_pre.push(shell_split(v)),
                ("service", "ExecStartPost") => def.exec_start_post.push(shell_split(v)),
                ("service", "ExecStop") => def.exec_stop = shell_split(v),
                ("service", "ExecReload") => def.exec_reload = shell_split(v),
                ("service", "Restart") => {
                    def.restart = match v.to_lowercase().as_str() {
                        "no" => RestartPolicy::No,
                        "on-success" => RestartPolicy::OnSuccess,
                        "on-failure" => RestartPolicy::OnFailure,
                        "on-abnormal" => RestartPolicy::OnAbnormal,
                        "on-watchdog" => RestartPolicy::OnWatchdog,
                        "on-abort" => RestartPolicy::OnAbort,
                        "always" => RestartPolicy::Always,
                        _ => RestartPolicy::OnFailure,
                    };
                }
                ("service", "User") => def.user = Some(v.to_string()),
                ("service", "Group") => def.group = Some(v.to_string()),
                ("service", "WorkingDirectory") => def.working_directory = Some(PathBuf::from(v)),
                ("service", "Environment") => {
                    for e in shell_split(v) {
                        if let Some((ek, ev)) = e.split_once('=') {
                            def.environment.insert(ek.to_string(), ev.to_string());
                        }
                    }
                }
                ("service", "PIDFile") => def.pid_file = Some(PathBuf::from(v)),
                ("service", "TimeoutStartSec") => def.timeout_start = parse_sec(v),
                ("service", "TimeoutStopSec") => def.timeout_stop = parse_sec(v),
                ("install", "WantedBy") => def.wanted_by.extend(split_list(v)),
                ("install", "RequiredBy") => def.required_by.extend(split_list(v)),
                _ => {}
            }
        }
        if def.exec.is_empty() {
            return Err(anyhow::anyhow!("systemd unit has no ExecStart"));
        }
        Ok(def)
    }

    pub fn load_openrc(&self, name: &str) -> Result<ServiceDefinition> {
        let path = format!("/etc/init.d/{}", name);
        if !Path::new(&path).exists() {
            return Err(anyhow::anyhow!("openrc script not found"));
        }
        let content = fs::read_to_string(&path)?;
        self.parse_openrc(&content, name)
    }

    fn parse_openrc(&self, content: &str, name: &str) -> Result<ServiceDefinition> {
        let mut def = ServiceDefinition::default();
        def.name = name.to_string();
        def.type_ = ServiceType::Forking;

        for line in content.lines() {
            let line = line.trim();
            if let Some(v) = line.strip_prefix("description=") {
                def.description = v.trim_matches('"').trim_matches('\'').to_string();
            } else if let Some(v) = line.strip_prefix("command=") {
                let cmd = v.trim_matches('"').trim_matches('\'');
                def.exec = shell_split(cmd);
            } else if let Some(v) = line.strip_prefix("command_args=") {
                let args = v.trim_matches('"').trim_matches('\'');
                def.exec.extend(shell_split(args));
            } else if let Some(v) = line.strip_prefix("pidfile=") {
                def.pid_file = Some(PathBuf::from(v.trim_matches('"').trim_matches('\'')));
            } else if let Some(v) = line.strip_prefix("user=") {
                def.user = Some(v.trim_matches('"').trim_matches('\'').to_string());
            } else if let Some(v) = line.strip_prefix("group=") {
                def.group = Some(v.trim_matches('"').trim_matches('\'').to_string());
            } else if line.starts_with("depend()") || line.starts_with("need ") || line.starts_with("\tneed ") {
                // minimal depends parsing: need net logger etc
                let deps = line.replace("need", " ").replace("use", " ").replace("after", " ").replace("before", " ");
                for d in deps.split_whitespace() {
                    if d == "depend()" || d == "{" || d == "}" { continue; }
                    def.after.push(d.to_string());
                }
            }
        }
        if def.exec.is_empty() {
            // fallback: /etc/init.d/$name is executable itself with start arg
            def.exec = vec![format!("/etc/init.d/{}", name), "start".into()];
            def.type_ = ServiceType::Oneshot;
        }
        Ok(def)
    }

    pub fn load_runit(&self, name: &str) -> Result<ServiceDefinition> {
        let paths = [
            format!("/etc/sv/{}/run", name),
            format!("/run/runit/service/{}/run", name),
            format!("/var/service/{}/run", name),
        ];
        for p in &paths {
            if Path::new(p).exists() {
                let content = fs::read_to_string(p)?;
                return self.parse_runit(&content, name, p);
            }
        }
        Err(anyhow::anyhow!("runit service not found"))
    }

    fn parse_runit(&self, content: &str, name: &str, run_path: &str) -> Result<ServiceDefinition> {
        let mut def = ServiceDefinition::default();
        def.name = name.to_string();
        def.description = format!("runit service {}", name);
        def.type_ = ServiceType::Simple;
        def.restart = RestartPolicy::Always;
        // runit run script is usually: exec chpst ... daemon
        // We wrap it: /bin/sh run_path
        def.exec = vec!["/bin/sh".to_string(), run_path.to_string()];
        // try to extract plain exec line
        for line in content.lines() {
            let l = line.trim();
            if let Some(cmd) = l.strip_prefix("exec ") {
                def.exec = shell_split(cmd);
                break;
            }
        }
        Ok(def)
    }

    pub fn load_s6(&self, name: &str) -> Result<ServiceDefinition> {
        let dir_paths = [
            format!("/etc/s6/sv/{}", name),
            format!("/etc/s6/services/{}", name),
        ];
        for d in &dir_paths {
            let run = Path::new(d).join("run");
            if run.exists() {
                let content = fs::read_to_string(&run)?;
                let mut def = self.parse_runit(&content, name, run.to_str().unwrap())?;
                def.description = format!("s6 service {}", name);
                return Ok(def);
            }
        }
        Err(anyhow::anyhow!("s6 service not found"))
    }
}

fn split_list(v: &str) -> Vec<String> {
    v.split_whitespace().map(|s| s.to_string()).collect()
}

fn strip_prefix_chars<'a>(v: &'a str, chars: &[char]) -> &'a str {
    let mut s = v.trim();
    loop {
        let mut stripped = false;
        for c in chars {
            if let Some(rest) = s.strip_prefix(*c) {
                s = rest.trim_start();
                stripped = true;
                break;
            }
        }
        if !stripped { break; }
    }
    s
}

fn shell_split(s: &str) -> Vec<String> {
    // minimal shell-like split respecting single/double quotes
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '\\' if in_double || (!in_single && !in_double) => {
                if let Some(n) = chars.next() { cur.push(n); }
            }
            ' ' | '\t' if !in_single && !in_double => {
                if !cur.is_empty() { out.push(cur.clone()); cur.clear(); }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() { out.push(cur); }
    out
}

fn parse_sec(v: &str) -> u64 {
    let v = v.trim();
    if let Some(n) = v.strip_suffix('s') { return n.parse().unwrap_or(90); }
    if let Some(n) = v.strip_suffix("sec") { return n.parse().unwrap_or(90); }
    if let Some(n) = v.strip_suffix("min") { return n.trim().parse::<u64>().unwrap_or(1) * 60; }
    v.parse().unwrap_or(90)
}
