use crate::config::RepoConfig;
use anyhow::Result;
use bleepkg::PackageIndex;
use reqwest::Client;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Repository {
    config: RepoConfig,
    client: Client,
    cache_dir: PathBuf,
}

impl Repository {
    pub fn new(config: RepoConfig, cache_dir: PathBuf) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(format!("bleepm/{}", env!("CARGO_PKG_VERSION")))
            .build()?;
        std::fs::create_dir_all(&cache_dir).ok();
        Ok(Self { config, client, cache_dir })
    }

    pub async fn fetch_index(&self) -> Result<PackageIndex> {
        if let Some(dir) = self.config.url.strip_prefix("file://") {
            let data = std::fs::read(format!("{}/index.json.zst", dir.trim_end_matches('/')))?;
            let decompressed = zstd::decode_all(&data[..])?;
            return Ok(serde_json::from_slice(&decompressed)?);
        }
        let url = format!("{}/index.json.zst", self.config.url.trim_end_matches('/'));
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Failed to fetch index: {}", resp.status()));
        }
        let data = resp.bytes().await?;
        let decompressed = zstd::decode_all(&data[..])?;
        Ok(serde_json::from_slice(&decompressed)?)
    }

    pub async fn download_package(&self, pkg: &bleepkg::PackageMetadata) -> Result<PathBuf> {
        let filename = pkg.package_filename();
        let dest = self.cache_dir.join(&filename);
        if dest.exists() {
            return Ok(dest);
        }
        if let Some(dir) = self.config.url.strip_prefix("file://") {
            let src = format!("{}/{}", dir.trim_end_matches('/'), filename);
            std::fs::create_dir_all(&self.cache_dir).ok();
            std::fs::copy(&src, &dest)?;
            return Ok(dest);
        }
        let url = format!("{}/{}", self.config.url.trim_end_matches('/'), filename);
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Download failed: {}", resp.status()));
        }
        let data = resp.bytes().await?;
        tokio::fs::write(&dest, &data).await?;
        Ok(dest)
    }
}
