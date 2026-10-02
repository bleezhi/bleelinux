use super::*;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs::File;
use sha2::{Sha256, Digest};

/// Unpack a .bleepkg payload under `root`, skipping embedded metadata files.
pub fn extract_archive(archive_path: &Path, root: &Path) -> Result<()> {
    let file = File::open(archive_path)?;
    let mut decoder = zstd::stream::Decoder::new(file)?;
    let mut magic = [0u8; 8];
    decoder.read_exact(&mut magic)?;
    if magic != PACKAGE_MAGIC {
        return Err(BleepkgError::Invalid("Not a valid bleepkg archive".into()));
    }
    let mut archive = tar::Archive::new(decoder);
    let entries = archive.entries().map_err(|e| BleepkgError::Archive(e.to_string()))?;
    for entry in entries {
        let mut entry = entry.map_err(|e| BleepkgError::Archive(e.to_string()))?;
        let raw = entry.path().map_err(|e| BleepkgError::Archive(e.to_string()))?.to_path_buf();
        let rel_str = raw.to_string_lossy().trim_start_matches("./").to_string();
        if rel_str.is_empty() || rel_str == "." { continue; }
        // skip embedded metadata / signature sidecars
        if let Some(name) = Path::new(&rel_str).file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') { continue; }
        }
        let rel = Path::new(&rel_str);
        if rel.is_absolute() || rel.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
            continue;
        }
        let dest = root.join(rel);
        let kind = entry.header().entry_type();
        if kind.is_dir() {
            fs::create_dir_all(&dest)?;
        } else if kind.is_symlink() {
            if let Some(parent) = dest.parent() { fs::create_dir_all(parent)?; }
            if dest.exists() || dest.is_symlink() { fs::remove_file(&dest).ok(); }
            if let Some(target) = entry.link_name().map_err(|e| BleepkgError::Archive(e.to_string()))? {
                symlink(target.as_ref(), &dest)?;
            }
        } else {
            if let Some(parent) = dest.parent() { fs::create_dir_all(parent)?; }
            entry.unpack(&dest).map_err(|e| BleepkgError::Archive(e.to_string()))?;
            if let Ok(mode) = entry.header().mode() {
                fs::set_permissions(&dest, fs::Permissions::from_mode(mode)).ok();
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallPlan {
    pub install: Vec<PackageMetadata>,
    pub upgrade: Vec<(PackageMetadata, PackageMetadata)>,  // (old, new)
    pub remove: Vec<PackageMetadata>,
    pub download_size: u64,
    pub installed_size: u64,
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub plan: InstallPlan,
    pub db: PackageDatabase,
    pub dry_run: bool,
    pub package_dir: Option<PathBuf>,
    pub root: PathBuf,
}

impl Transaction {
    pub fn new(plan: InstallPlan, db: PackageDatabase, dry_run: bool) -> Self {
        Self { plan, db, dry_run, package_dir: None, root: PathBuf::from("/") }
    }

    pub fn with_package_dir(mut self, dir: PathBuf) -> Self {
        self.package_dir = Some(dir);
        self
    }

    pub fn with_root(mut self, root: PathBuf) -> Self {
        self.root = root;
        self
    }

    pub fn execute(&mut self) -> Result<()> {
        // Phase 1: Pre-transaction hooks
        self.run_hooks("pre-transaction")?;

        // Phase 2: Remove packages
        for pkg in &self.plan.remove {
            self.remove_package(pkg)?;
        }

        // Phase 3: Install/upgrade packages
        for pkg in &self.plan.install {
            self.install_package(pkg)?;
        }

        for (old, new) in &self.plan.upgrade {
            self.upgrade_package(old, new)?;
        }

        // Phase 4: Post-transaction hooks
        self.run_hooks("post-transaction")?;

        // Phase 5: Update database
        if !self.dry_run {
            self.db.commit(&self.plan)?;
        }

        Ok(())
    }

    fn install_package(&self, pkg: &PackageMetadata) -> Result<()> {
        if self.dry_run {
            println!("Would install: {}", pkg.package_filename());
            return Ok(());
        }

        println!("Installing: {}", pkg.package_filename());

        // Extract payload from downloaded archive when available
        if let Some(dir) = &self.package_dir {
            let archive = dir.join(pkg.package_filename());
            if archive.exists() {
                extract_archive(&archive, &self.root)?;
            } else {
                println!("  warn: archive not in cache, recording files only");
                for file in &pkg.files {
                    if file.is_ghost || file.is_symlink { continue; }
                    let dest = self.root.join(file.path.strip_prefix("/").unwrap_or(&file.path));
                    if let Some(parent) = dest.parent() {
                        fs::create_dir_all(parent)?;
                    }
                }
            }
        } else {
            // No cache dir: ensure parent dirs exist (DB-only record)
            for file in &pkg.files {
                if let Some(parent) = file.path.parent() {
                    let dir = self.root.join(parent.strip_prefix("/").unwrap_or(parent));
                    fs::create_dir_all(dir)?;
                }
            }
        }

        // Restore symlinks from metadata (also covered by tar, but ensure)
        for file in &pkg.files {
            if !file.is_symlink { continue; }
            if file.is_ghost { continue; }
            if let Some(target) = &file.symlink_target {
                let dest = self.root.join(file.path.strip_prefix("/").unwrap_or(&file.path));
                if dest.exists() || dest.is_symlink() {
                    fs::remove_file(&dest).ok();
                }
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)?;
                }
                symlink(target, &dest)?;
            }
        }

        // Run post-install scripts
        self.run_pkg_script(pkg, "post-install")?;

        Ok(())
    }

    fn upgrade_package(&self, old: &PackageMetadata, new: &PackageMetadata) -> Result<()> {
        if self.dry_run {
            println!("Would upgrade: {} -> {}", old.full_version(), new.full_version());
            return Ok(());
        }

        println!("Upgrading: {} -> {}", old.full_version(), new.full_version());
        
        // Run pre-upgrade
        self.run_pkg_script(old, "pre-upgrade")?;
        
        // Remove old files not in new
        let old_files: HashMap<_, _> = old.files.iter().map(|f| (&f.path, f)).collect();
        let new_files: HashMap<_, _> = new.files.iter().map(|f| (&f.path, f)).collect();
        
        for (path, _) in old_files {
            if !new_files.contains_key(path) && !path.to_string_lossy().starts_with("/etc/") {
                // Remove file (but keep config files)
                let dest = Path::new("/").join(path.strip_prefix("/").unwrap_or(path));
                if dest.exists() {
                    fs::remove_file(dest)?;
                }
            }
        }

        // Install new files
        self.install_package(new)?;
        
        // Run post-upgrade
        self.run_pkg_script(new, "post-upgrade")?;

        Ok(())
    }

    fn remove_package(&self, pkg: &PackageMetadata) -> Result<()> {
        if self.dry_run {
            println!("Would remove: {}", pkg.package_filename());
            return Ok(());
        }

        println!("Removing: {}", pkg.package_filename());
        
        // Run pre-remove
        self.run_pkg_script(pkg, "pre-remove")?;

        // Remove files (except config modified by user)
        for file in &pkg.files {
            if file.is_config {
                // Check if modified
                let dest = Path::new("/").join(file.path.strip_prefix("/").unwrap_or(&file.path));
                if dest.exists() {
                    let current_hash = compute_file_hash(&dest)?;
                    if current_hash != file.sha256 {
                        println!("  Keeping modified config: {}", file.path.display());
                        continue;
                    }
                }
            }
            
            if !file.is_ghost {
                let dest = Path::new("/").join(file.path.strip_prefix("/").unwrap_or(&file.path));
                if dest.exists() {
                    if file.is_symlink || dest.is_file() {
                        fs::remove_file(dest)?;
                    } else if dest.is_dir() {
                        // Only remove empty dirs
                        if fs::read_dir(&dest)?.next().is_none() {
                            fs::remove_dir(dest)?;
                        }
                    }
                }
            }
        }

        // Run post-remove
        self.run_pkg_script(pkg, "post-remove")?;

        Ok(())
    }

    fn run_pkg_script(&self, pkg: &PackageMetadata, script: &str) -> Result<()> {
        // Scripts would be stored in /var/lib/bleepkg/scripts/<pkg>/<script>
        let script_path = Path::new("/var/lib/bleepkg/scripts")
            .join(&pkg.name)
            .join(script);
        
        if script_path.exists() {
            println!("  Running {script} for {}", pkg.name);
            let status = Command::new("/bin/sh")
                .arg(&script_path)
                .status()?;
            if !status.success() {
                return Err(BleepkgError::Dependency(
                    format!("Script {script} failed with {:?}", status.code())
                ));
            }
        }
        Ok(())
    }

    fn run_hooks(&self, hook: &str) -> Result<()> {
        let hook_dir = Path::new("/etc/bleepkg/hooks").join(hook);
        if hook_dir.exists() {
            for entry in fs::read_dir(hook_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map(|s| s == "sh").unwrap_or(false) {
                    println!("  Running hook: {}", path.display());
                    let status = Command::new("/bin/sh").arg(&path).status()?;
                    if !status.success() {
                        return Err(BleepkgError::Dependency(
                            format!("Hook {hook} failed with {:?}", status.code())
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

fn compute_file_hash(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

#[derive(Debug, Clone)]
pub struct PackageDatabase {
    db_path: PathBuf,
}

impl PackageDatabase {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db_path = path.as_ref().to_path_buf();
        if let Some(parent) = db_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let conn = rusqlite::Connection::open(&db_path)?;
        conn.execute_batch(include_str!("../schema.sql"))?;
        Ok(Self { db_path })
    }

    fn conn(&self) -> std::result::Result<rusqlite::Connection, rusqlite::Error> {
        rusqlite::Connection::open(&self.db_path)
    }

    pub fn commit(&mut self, plan: &InstallPlan) -> Result<()> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;

        for pkg in &plan.install {
            Self::insert_package_static(&tx, pkg)?;
        }

        for (old, new) in &plan.upgrade {
            Self::remove_package_static(&tx, &old.name)?;
            Self::insert_package_static(&tx, new)?;
        }

        for pkg in &plan.remove {
            Self::remove_package_static(&tx, &pkg.name)?;
        }

        tx.commit()?;
        Ok(())
    }

    fn insert_package(&self, tx: &rusqlite::Transaction, pkg: &PackageMetadata) -> Result<()> {
        Self::insert_package_static(tx, pkg)
    }

    fn insert_package_static(tx: &rusqlite::Transaction, pkg: &PackageMetadata) -> Result<()> {
        tx.execute(
            "INSERT INTO packages (id, name, version, release, epoch, arch, libc, metadata, installed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(name) DO UPDATE SET
                version=excluded.version,
                release=excluded.release,
                epoch=excluded.epoch,
                arch=excluded.arch,
                libc=excluded.libc,
                metadata=excluded.metadata,
                installed_at=excluded.installed_at",
            (
                pkg.id.to_string(),
                &pkg.name,
                pkg.version.to_string(),
                pkg.release as i64,
                pkg.epoch as i64,
                pkg.arch.to_string(),
                pkg.libc.to_string(),
                serde_json::to_string(pkg)?,
                Utc::now().to_rfc3339(),
            ),
        )?;

        for file in &pkg.files {
            tx.execute(
                "INSERT INTO files (package_id, path, size, mode, sha256, blake3, is_config, is_ghost)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    pkg.id.to_string(),
                    file.path.to_string_lossy().to_string(),
                    file.size as i64,
                    file.mode as i32,
                    &file.sha256,
                    &file.blake3,
                    file.is_config,
                    file.is_ghost,
                ),
            )?;
        }

        for dep in &pkg.dependencies {
            tx.execute(
                "INSERT INTO dependencies (package_id, dep_name, version_req, optional)
                 VALUES (?, ?, ?, ?)",
                (
                    pkg.id.to_string(),
                    &dep.name,
                    &dep.version_req,
                    dep.optional,
                ),
            )?;
        }

        Ok(())
    }

    fn remove_package(&self, tx: &rusqlite::Transaction, name: &str) -> Result<()> {
        Self::remove_package_static(tx, name)
    }

    fn remove_package_static(tx: &rusqlite::Transaction, name: &str) -> Result<()> {
        tx.execute("DELETE FROM files WHERE package_id IN (SELECT id FROM packages WHERE name = ?)", [name])?;
        tx.execute("DELETE FROM dependencies WHERE package_id IN (SELECT id FROM packages WHERE name = ?)", [name])?;
        tx.execute("DELETE FROM packages WHERE name = ?", [name])?;
        Ok(())
    }

    pub fn get_installed(&self, name: &str) -> Result<Option<PackageMetadata>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT metadata FROM packages WHERE name = ? ORDER BY installed_at DESC LIMIT 1")?;
        let mut rows = stmt.query([name])?;
        
        if let Some(row) = rows.next()? {
            let metadata: String = row.get(0)?;
            Ok(Some(serde_json::from_str(&metadata).map_err(|e| BleepkgError::Serde(e))?))
        } else {
            Ok(None)
        }
    }

    pub fn list_installed(&self) -> Result<Vec<PackageMetadata>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT metadata FROM packages ORDER BY name")?;
        let rows = stmt.query_map([], |row| {
            let metadata: String = row.get(0)?;
            Ok(serde_json::from_str(&metadata).unwrap())
        })?;
        
        let mut pkgs = Vec::new();
        for pkg in rows {
            pkgs.push(pkg?);
        }
        Ok(pkgs)
    }
}