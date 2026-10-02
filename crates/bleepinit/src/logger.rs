use anyhow::Result;

pub fn init_logging(level: &str) -> Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_new(level).unwrap_or_else(|_| "info".into());
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
    Ok(())
}
