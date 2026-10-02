use anyhow::Result;
use std::os::unix::net::UnixListener;
use std::path::Path;

pub fn bind_unix(path: &str) -> Result<UnixListener> {
    if Path::new(path).exists() {
        std::fs::remove_file(path).ok();
    }
    Ok(UnixListener::bind(path)?)
}
