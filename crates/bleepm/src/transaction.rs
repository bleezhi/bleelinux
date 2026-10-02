use crate::InstallPlan;
use anyhow::Result;
use bleepkg::install::PackageDatabase;
use std::path::PathBuf;

pub struct Transaction {
    pub plan: InstallPlan,
    pub db: PackageDatabase,
    pub dry_run: bool,
    pub package_dir: Option<PathBuf>,
}

impl Transaction {
    pub fn new(plan: InstallPlan, db: PackageDatabase, dry_run: bool) -> Self {
        Self { plan, db, dry_run, package_dir: None }
    }

    pub fn with_package_dir(mut self, dir: PathBuf) -> Self {
        self.package_dir = Some(dir);
        self
    }

    pub fn execute(&mut self) -> Result<()> {
        let bplan = bleepkg::install::InstallPlan {
            install: self.plan.install.clone(),
            upgrade: self.plan.upgrade.clone(),
            remove: self.plan.remove.clone(),
            download_size: self.plan.download_size,
            installed_size: self.plan.installed_size,
        };
        let mut inner = bleepkg::install::Transaction::new(bplan, self.db.clone(), self.dry_run);
        if let Some(dir) = &self.package_dir {
            inner = inner.with_package_dir(dir.clone());
        }
        inner.execute().map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(())
    }
}
