use anyhow::Result;
use std::path::Path;

pub fn ensure_slices() -> Result<()> {
    for slice in ["system.slice", "user.slice", "machine.slice"] {
        let p = format!("/sys/fs/cgroup/{}", slice);
        if Path::new("/sys/fs/cgroup").exists() {
            std::fs::create_dir_all(&p).ok();
        }
    }
    Ok(())
}

pub fn move_self_to_init_scope() -> Result<()> {
    // PID 1 stays in root cgroup; nothing to do
    Ok(())
}
