pub mod resolver;
pub mod repo;
pub mod source;
pub mod cli;
pub mod config;
pub mod hooks;
pub mod transaction;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;
use bleepkg::{PackageMetadata, Architecture, Libc, Dependency, PackageIndex, BuildRecipe};
use tokio::sync::RwLock;

pub use resolver::*;
pub use repo::Repository;
pub use source::*;
pub use cli::*;
pub use config::*;
pub use transaction::Transaction as BleepTransaction;

#[derive(Debug, Clone)]
pub struct PackageManager {
    pub config: Config,
    pub config_path: Option<PathBuf>,
    pub repo_cache: Arc<RwLock<HashMap<String, PackageIndex>>>,
    pub installed_db: bleepkg::install::PackageDatabase,
}

impl PackageManager {
    pub fn new(config: Config) -> Result<Self> {
        let installed_db = bleepkg::install::PackageDatabase::new(&config.db_path)?;
        Ok(Self {
            config,
            config_path: None,
            repo_cache: Arc::new(RwLock::new(HashMap::new())),
            installed_db,
        })
    }

    pub fn with_config_path(mut self, path: PathBuf) -> Self {
        self.config_path = Some(path);
        self
    }

    fn persist(&self) -> Result<()> {
        if let Some(p) = &self.config_path {
            self.config.save_to(p)?;
        } else {
            self.config.save()?;
        }
        Ok(())
    }

    pub async fn refresh_repos(&self) -> Result<()> {
        let mut cache = self.repo_cache.write().await;
        for repo_cfg in &self.config.repos {
            if !repo_cfg.enabled { continue; }
            let r = Repository::new(repo_cfg.clone(), self.config.cache_dir.clone())?;
            match r.fetch_index().await {
                Ok(index) => { cache.insert(repo_cfg.name.clone(), index); }
                Err(e) => eprintln!("failed to fetch {}: {}", repo_cfg.name, e),
            }
        }
        Ok(())
    }

    pub async fn search(&self, query: &str) -> Result<Vec<PackageMetadata>> {
        let cache = self.repo_cache.read().await;
        let mut results = Vec::new();
        for index in cache.values() {
            for versions in index.packages.values() {
                for pkg in versions {
                    if pkg.name.contains(query)
                        || pkg.description.contains(query)
                        || pkg.summary.contains(query) {
                        results.push(pkg.clone());
                    }
                }
            }
        }
        results.sort_by(|a, b| a.name.cmp(&b.name));
        results.dedup_by(|a, b| a.name == b.name && a.version == b.version);
        Ok(results)
    }

    pub async fn resolve_install(&self, packages: &[String]) -> Result<InstallPlan> {
        let cache = self.repo_cache.read().await;
        let mut resolver = DependencyResolver::new(&cache, &self.installed_db);
        for pkg_spec in packages {
            resolver.add_request(pkg_spec)?;
        }
        resolver.resolve()
    }

    pub async fn install(&self, packages: &[String], dry_run: bool, assume_yes: bool) -> Result<()> {
        let plan = self.resolve_install(packages).await?;
        println!("Transaction Summary:");
        println!("  Install: {}", plan.install.len());
        println!("  Upgrade: {}", plan.upgrade.len());
        println!("  Remove:  {}", plan.remove.len());
        println!("  Download: {}", format_size(plan.download_size));
        println!("  Disk space: {}", format_size(plan.installed_size));
        if dry_run {
            println!("\nDry run - no changes made");
            return Ok(());
        }
        if !assume_yes {
            print!("Proceed? [Y/n]: ");
            use std::io::{self, Write};
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            if !input.trim().is_empty() && !input.trim().eq_ignore_ascii_case("y") {
                println!("Aborted");
                return Ok(());
            }
        }
        self.download_plan(&plan).await?;
        let mut tx = crate::transaction::Transaction::new(plan, self.installed_db.clone(), false)
            .with_package_dir(self.config.cache_dir.clone());
        tx.execute()?;
        println!("Transaction completed successfully");
        Ok(())
    }

    pub async fn build_from_source(&self, pkg_name: &str, git_url: Option<String>) -> Result<PackageMetadata> {
        let recipe = self.find_recipe(pkg_name)?;
        let mut recipe = recipe;
        if let Some(url) = git_url {
            recipe.source = bleepkg::metadata::SourceSpec::Git {
                url,
                rev: "HEAD".to_string(),
                submodules: true,
            };
        }
        let builder = source::SourceBuilder::new(self.config.clone());
        builder.build(&recipe).await
    }

    fn find_recipe(&self, name: &str) -> Result<BuildRecipe> {
        let recipe_path = self.config.recipes_dir.join(name).join("recipe.toml");
        if recipe_path.exists() {
            let content = std::fs::read_to_string(recipe_path)?;
            return Ok(toml::from_str(&content)?);
        }
        Err(anyhow::anyhow!("Recipe not found: {}", name))
    }

    pub async fn remove(&self, packages: &[String], _recursive: bool, _purge: bool, dry_run: bool) -> Result<()> {
        let mut to_remove = Vec::new();
        for name in packages {
            if let Some(pkg) = self.installed_db.get_installed(name)? {
                to_remove.push(pkg);
            }
        }
        let plan = InstallPlan {
            install: vec![],
            upgrade: vec![],
            remove: to_remove,
            download_size: 0,
            installed_size: 0,
        };
        if dry_run {
            for pkg in &plan.remove {
                println!("Would remove: {}", pkg.name);
            }
            return Ok(());
        }
        let mut tx = crate::transaction::Transaction::new(plan, self.installed_db.clone(), false);
        tx.execute()?;
        Ok(())
    }

    pub async fn upgrade(&self, packages: &[String], _full: bool, dry_run: bool) -> Result<()> {
        self.refresh_repos().await?;
        let to_upgrade = if packages.is_empty() {
            self.installed_db.list_installed()?
        } else {
            let mut v = Vec::new();
            for n in packages {
                if let Some(p) = self.installed_db.get_installed(n)? {
                    v.push(p);
                }
            }
            v
        };
        let names: Vec<String> = to_upgrade.iter().map(|p| p.name.clone()).collect();
        let plan = self.resolve_install(&names).await?;
        if plan.install.is_empty() && plan.upgrade.is_empty() {
            println!("Nothing to upgrade");
            return Ok(());
        }
        if dry_run {
            println!("Would upgrade {} packages", plan.upgrade.len());
            return Ok(());
        }
        self.download_plan(&plan).await?;
        let mut tx = crate::transaction::Transaction::new(plan, self.installed_db.clone(), false)
            .with_package_dir(self.config.cache_dir.clone());
        tx.execute()?;
        Ok(())
    }

    pub async fn show_info(&self, name: &str, show_files: bool, show_deps: bool) -> Result<()> {
        if let Some(pkg) = self.installed_db.get_installed(name)? {
            println!("Installed: {} {}", pkg.name, pkg.full_version());
            println!("  Description: {}", pkg.description);
            println!("  License: {}", pkg.license);
            println!("  Size: {}", format_size(pkg.installed_size));
            if show_files {
                for f in &pkg.files {
                    println!("  {}", f.path.display());
                }
            }
            if show_deps {
                for d in &pkg.dependencies {
                    println!("  Depends: {} ({})", d.name, d.version_req);
                }
            }
        } else {
            let cache = self.repo_cache.read().await;
            for index in cache.values() {
                if let Some(versions) = index.packages.get(name) {
                    for pkg in versions {
                        println!("Available: {} {}", pkg.name, pkg.full_version());
                        println!("  Description: {}", pkg.description);
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn list_installed(&self, _explicit: bool, show_size: bool) -> Result<()> {
        let pkgs = self.installed_db.list_installed()?;
        for pkg in pkgs {
            if show_size {
                println!("{} {} ({})", pkg.name, pkg.full_version(), format_size(pkg.installed_size));
            } else {
                println!("{} {}", pkg.name, pkg.full_version());
            }
        }
        Ok(())
    }

    pub async fn repo_command(&mut self, action: RepoAction) -> Result<()> {
        match action {
            RepoAction::List => {
                for r in &self.config.repos {
                    println!("{} {} [{}]", r.name, r.url, if r.enabled { "enabled" } else { "disabled" });
                }
            }
            RepoAction::Add { name, url } => {
                if self.config.repos.iter().any(|r| r.name == name) {
                    return Err(anyhow::anyhow!("repo {} already exists", name));
                }
                self.config.repos.push(RepoConfig {
                    name: name.clone(),
                    url: url.clone(),
                    enabled: true,
                    arch: self.config.target_arch.clone(),
                    libc: self.config.target_libc.clone(),
                    priority: 50,
                    verify_signatures: self.config.verify_signatures,
                    public_key: None,
                    recipes_url: None,
                });
                self.persist()?;
                println!("added repo {} -> {}", name, url);
            }
            RepoAction::Remove { name } => {
                let n = self.config.repos.len();
                self.config.repos.retain(|r| r.name != name);
                if self.config.repos.len() == n {
                    return Err(anyhow::anyhow!("repo {} not found", name));
                }
                self.persist()?;
                println!("removed repo {}", name);
            }
            RepoAction::Enable { name } => {
                let r = self.config.repos.iter_mut().find(|r| r.name == name)
                    .ok_or_else(|| anyhow::anyhow!("repo {} not found", name))?;
                r.enabled = true;
                self.persist()?;
                println!("enabled repo {}", name);
            }
            RepoAction::Disable { name } => {
                let r = self.config.repos.iter_mut().find(|r| r.name == name)
                    .ok_or_else(|| anyhow::anyhow!("repo {} not found", name))?;
                r.enabled = false;
                self.persist()?;
                println!("disabled repo {}", name);
            }
            RepoAction::Refresh => {
                self.refresh_repos().await?;
                println!("Repositories refreshed");
            }
        }
        Ok(())
    }

    pub async fn verify(&self, package: Option<&str>, _fix: bool) -> Result<()> {
        let verifier = bleepkg::verify::Verifier::new(self.config.trusted_keys.clone(), self.config.verify_signatures);
        if let Some(name) = package {
            let result = verifier.verify_installed(&self.installed_db, name)?;
            if result.is_valid() {
                println!("{} - OK", name);
            } else {
                println!("{} - FAILED", name);
                for e in &result.errors {
                    println!("  {}", e);
                }
            }
        } else {
            for pkg in self.installed_db.list_installed()? {
                let result = verifier.verify_installed(&self.installed_db, &pkg.name)?;
                if !result.is_valid() {
                    println!("{} - FAILED", pkg.name);
                    for e in &result.errors {
                        println!("  {}", e);
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn clean(&self, packages: bool, build: bool, all: bool) -> Result<()> {
        if packages || all {
            let _ = std::fs::remove_dir_all(&self.config.cache_dir);
            println!("Cleaned package cache");
        }
        if build || all {
            let _ = std::fs::remove_dir_all(&self.config.build_dir);
            println!("Cleaned build directory");
        }
        Ok(())
    }

    pub async fn show_history(&self) -> Result<()> {
        println!("Transaction history not yet implemented");
        Ok(())
    }

    pub fn pack(&self, dir: &std::path::Path, out: &std::path::Path, name: &str, version: &str) -> Result<PackageMetadata> {
        use bleepkg::metadata::{Architecture, Libc};
        use std::str::FromStr;
        let ver = version.parse()
            .map_err(|e| anyhow::anyhow!("invalid version {}: {}", version, e))?;
        let arch = Architecture::from_str(&self.config.target_triplet.split('-').next().unwrap_or("x86_64"))
            .unwrap_or(Architecture::X86_64);
        let libc = if self.config.target_triplet.contains("musl") { Libc::Musl } else { Libc::Glibc };
        let mut builder = bleepkg::archive::PackageBuilder::new(name, ver, arch, libc);
        // If `out` is a directory, write the canonical filename inside it.
        let out_path = if out.is_dir() {
            let m = builder.metadata_mut();
            let fname = format!("{}-{}-{}.{}.bin.bleepkg", name, m.full_version(), m.arch, m.libc);
            out.join(fname)
        } else {
            out.to_path_buf()
        };
        builder.metadata_mut().description = format!("{} (packed from {})", name, dir.display());
        builder.metadata_mut().summary = builder.metadata_mut().description.clone();
        let mut files: Vec<std::path::PathBuf> = walkdir::WalkDir::new(dir)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file() || e.file_type().is_symlink())
            .map(|e| e.path().to_path_buf())
            .collect();
        files.sort();
        for src in files {
            let rel = src.strip_prefix(dir)?;
            let dest = std::path::PathBuf::from("/").join(rel);
            if src.is_symlink() {
                builder.add_symlink(std::fs::read_link(&src)?, dest)?;
            } else {
                use std::os::unix::fs::PermissionsExt;
                let mode = std::fs::metadata(&src)?.permissions().mode();
                builder.add_file(&src, &dest, mode)?;
            }
        }
        let meta = builder.build(&out_path)?;
        println!("packed {}", meta.package_filename());
        Ok(meta)
    }

    pub fn repo_index(&self, dir: &std::path::Path) -> Result<()> {
        use std::collections::HashMap;
        let mut packages: HashMap<String, Vec<PackageMetadata>> = HashMap::new();
        let mut entries: Vec<std::path::PathBuf> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map(|x| x == "bleepkg").unwrap_or(false))
            .collect();
        entries.sort();
        for p in &entries {
            let archive = bleepkg::archive::PackageArchive::open(p)?;
            // re-read real checksums of the file itself for the index
            let bytes = std::fs::read(p)?;
            let mut h1 = sha2::Sha256::new();
            use sha2::Digest;
            h1.update(&bytes);
            let mut m = archive.metadata.clone();
            m.checksum_sha256 = format!("{:x}", h1.finalize());
            m.checksum_blake3 = blake3::hash(&bytes).to_hex().to_string();
            m.download_size = bytes.len() as u64;
            packages.entry(m.name.clone()).or_default().push(m);
        }
        let index = PackageIndex {
            packages,
            generated: chrono::Utc::now(),
            repo_name: "local".into(),
            repo_url: format!("file://{}", dir.display()),
            arch: Architecture::X86_64,
            libc: Libc::Glibc,
        };
        let json = serde_json::to_vec(&index)?;
        let out = dir.join("index.json.zst");
        std::fs::write(&out, zstd::encode_all(&json[..], 19)?)?;
        // plain-JSON twin for the website (no zstd needed in the browser)
        std::fs::write(dir.join("packages.json"), serde_json::to_vec_pretty(&index)?)?;
        println!("index: {} ({} packages)", out.display(), index.packages.len());
        Ok(())
    }

    fn find_repo_in(cache: &HashMap<String, PackageIndex>, repos: &[RepoConfig], pkg: &PackageMetadata) -> Result<RepoConfig> {
        for (repo_name, index) in cache.iter() {
            if let Some(versions) = index.packages.get(&pkg.name) {
                if versions.iter().any(|v| v.version == pkg.version && v.release == pkg.release) {
                    return repos.iter()
                        .find(|r| &r.name == repo_name)
                        .cloned()
                        .ok_or_else(|| anyhow::anyhow!("repo {} not configured", repo_name));
                }
            }
        }
        Err(anyhow::anyhow!("no repo provides {}-{}", pkg.name, pkg.full_version()))
    }

    pub async fn download_plan(&self, plan: &InstallPlan) -> Result<()> {
        let pkgs: Vec<PackageMetadata> = plan.install.iter()
            .chain(plan.upgrade.iter().map(|(_, n)| n))
            .cloned()
            .collect();
        // resolve repo configs under a short read lock, download after
        let cfgs: Vec<(PackageMetadata, RepoConfig)> = {
            let cache = self.repo_cache.read().await;
            pkgs.iter().map(|p| Ok((p.clone(), Self::find_repo_in(&cache, &self.config.repos, p)?))).collect::<Result<Vec<_>>>()?
        };
        for (pkg, repo_cfg) in &cfgs {
            let r = Repository::new(repo_cfg.clone(), self.config.cache_dir.clone())?;
            let path = r.download_package(pkg).await?;
            println!("  downloaded {}", path.display());
        }
        Ok(())
    }
}

pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{:.1} {}", size, UNITS[unit])
    }
}

#[derive(Debug, Clone)]
pub struct InstallPlan {
    pub install: Vec<PackageMetadata>,
    pub upgrade: Vec<(PackageMetadata, PackageMetadata)>,
    pub remove: Vec<PackageMetadata>,
    pub download_size: u64,
    pub installed_size: u64,
}
