use anyhow::Result;
use std::path::Path;

pub fn install_hook(pkg_name: &str, hook_name: &str, script: &str) -> Result<()> {
    let hook_dir = Path::new("/var/lib/bleepkg/hooks").join(pkg_name);
    std::fs::create_dir_all(&hook_dir)?;
    std::fs::write(hook_dir.join(hook_name), script)?;
    Ok(())
}

pub fn install_system_hook(stage: &str, name: &str, script: &str) -> Result<()> {
    let hook_dir = Path::new("/etc/bleepkg/hooks").join(stage);
    std::fs::create_dir_all(&hook_dir)?;
    std::fs::write(hook_dir.join(format!("{name}.sh")), script)?;
    Ok(())
}

pub fn run_hook(hook_path: &Path) -> Result<()> {
    let status = std::process::Command::new("/bin/sh").arg(hook_path).status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("Hook failed: {:?}", status.code()));
    }
    Ok(())
}