use anyhow::{Result, Context};
use nix::mount::{mount, MsFlags};
use std::path::Path;

pub fn mount_all_from_fstab() -> Result<()> {
    let fstab = "/etc/fstab";
    if !Path::new(fstab).exists() { return Ok(()); }
    let content = std::fs::read_to_string(fstab)?;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 { continue; }
        let (src, target, fstype, opts) = (parts[0], parts[1], parts[2], parts[3]);
        if target == "/" || target == "/proc" || target == "/sys" || target == "/dev" { continue; }
        let flags = if opts.contains("ro") { MsFlags::MS_RDONLY } else { MsFlags::empty() };
        let _ = mount::<str, str, str, str>(Some(src), target, Some(fstype), flags, None);
    }
    Ok(())
}

pub fn remount_root_rw() -> Result<()> {
    let _ = mount::<str, str, str, str>(None::<&str>, "/", None::<&str>, MsFlags::MS_REMOUNT, None::<&str>)
        .context("remount / rw");
    Ok(())
}
