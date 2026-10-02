use crate::config::Config;
use anyhow::Result;
use bleepkg::{BuildRecipe, PackageMetadata, Architecture, Libc};
use std::path::PathBuf;

pub struct SourceBuilder {
    config: Config,
}

impl SourceBuilder {
    pub fn new(config: Config) -> Self {
        std::fs::create_dir_all(&config.build_dir).ok();
        Self { config }
    }

    pub async fn build(&self, recipe: &BuildRecipe) -> Result<PackageMetadata> {
        println!("Building {} from source (stub)...", recipe.pkg_name);
        let work = tempfile::tempdir()?;
        let src_dir = work.path().join("src");
        std::fs::create_dir_all(&src_dir)?;
        match &recipe.source {
            bleepkg::metadata::SourceSpec::Git { url, rev, .. } => {
                println!("  Cloning {} @ {}", url, rev);
                let status = std::process::Command::new("git").arg("clone").arg("--depth=1").arg(url).arg(&src_dir).status();
                if status.map(|s| !s.success()).unwrap_or(true) {
                    println!("  (clone failed or git missing, continuing with empty src)");
                }
            }
            bleepkg::metadata::SourceSpec::Url { url, .. } => {
                println!("  Would download {}", url);
            }
            bleepkg::metadata::SourceSpec::Local { path } => {
                println!("  Using local {}", path.display());
            }
        }
        Ok(PackageMetadata::from_recipe(recipe, Architecture::X86_64, Libc::Glibc))
    }
}

pub fn work_dir(config: &Config) -> PathBuf {
    config.build_dir.clone()
}
