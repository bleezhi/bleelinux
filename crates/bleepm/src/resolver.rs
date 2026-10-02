use std::collections::{HashMap, HashSet};
use anyhow::{Result, anyhow};
use bleepkg::{PackageMetadata, Architecture, Libc, PackageIndex};
use semver::VersionReq;
use crate::InstallPlan;

pub struct DependencyResolver<'a> {
    repo_cache: &'a HashMap<String, PackageIndex>,
    installed_db: &'a bleepkg::install::PackageDatabase,
    requests: Vec<PackageRequest>,
    target_arch: Architecture,
    target_libc: Libc,
}

#[derive(Debug, Clone)]
struct PackageRequest {
    name: String,
    version_req: Option<VersionReq>,
}

impl<'a> DependencyResolver<'a> {
    pub fn new(repo_cache: &'a HashMap<String, PackageIndex>, installed_db: &'a bleepkg::install::PackageDatabase) -> Self {
        Self { repo_cache, installed_db, requests: Vec::new(), target_arch: Architecture::X86_64, target_libc: Libc::Glibc }
    }

    pub fn add_request(&mut self, spec: &str) -> Result<()> {
        let spec = spec.trim().trim_end_matches("@git");
        let (name, version_req) = if let Some(idx) = spec.find(|c| c=='='||c=='>'||c=='<'||c=='~'||c=='^') {
            let n = spec[..idx].to_string();
            let req_str = spec[idx..].trim_start_matches('=').trim();
            let req_str = if req_str.is_empty() { "*" } else { req_str };
            (n, Some(VersionReq::parse(req_str)?))
        } else {
            (spec.to_string(), None)
        };
        self.requests.push(PackageRequest { name, version_req });
        Ok(())
    }

    pub fn resolve(&mut self) -> Result<InstallPlan> {
        let mut to_install = Vec::new();
        let mut to_upgrade = Vec::new();
        let to_remove = Vec::new();
        let mut seen = HashSet::new();
        let reqs = self.requests.clone();
        for r in &reqs {
            self.resolve_request(r, &mut to_install, &mut to_upgrade, &mut seen)?;
        }
        self.check_conflicts(&to_install, &to_upgrade)?;
        let download_size: u64 = to_install.iter().map(|p: &PackageMetadata| p.download_size).sum::<u64>()
            + to_upgrade.iter().map(|(_, n): &(PackageMetadata, PackageMetadata)| n.download_size).sum::<u64>();
        let installed_size: u64 = to_install.iter().map(|p: &PackageMetadata| p.installed_size).sum::<u64>()
            + to_upgrade.iter().map(|(_, n): &(PackageMetadata, PackageMetadata)| n.installed_size).sum::<u64>();
        Ok(InstallPlan { install: to_install, upgrade: to_upgrade, remove: to_remove, download_size, installed_size })
    }

    fn resolve_request(&self, request: &PackageRequest, to_install: &mut Vec<PackageMetadata>, to_upgrade: &mut Vec<(PackageMetadata, PackageMetadata)>, seen: &mut HashSet<String>) -> Result<()> {
        let key = format!("{}-{}", request.name, request.version_req.as_ref().map(|v| v.to_string()).unwrap_or_default());
        if seen.contains(&key) { return Ok(()); }
        seen.insert(key);
        let candidates = self.find_candidates(request)?;
        if candidates.is_empty() {
            return Err(anyhow!("No package found for {}", request.name));
        }
        let best = candidates.into_iter().max_by(|a,b| a.version.cmp(&b.version).then(a.release.cmp(&b.release))).unwrap();
        let best_clone = best.clone();
        if let Some(installed) = self.installed_db.get_installed(&best.name).map_err(|e| anyhow!("{}", e))? {
            if installed.version == best.version && installed.release == best.release {
                return Ok(());
            }
            to_upgrade.push((installed, best));
        } else {
            to_install.push(best_clone.clone());
        }
        for dep in &best_clone.dependencies {
            if dep.optional { continue; }
            let dep_req = PackageRequest { name: dep.name.clone(), version_req: VersionReq::parse(&dep.version_req).ok() };
            self.resolve_request(&dep_req, to_install, to_upgrade, seen)?;
        }
        Ok(())
    }

    fn find_candidates(&self, request: &PackageRequest) -> Result<Vec<PackageMetadata>> {
        let mut out = Vec::new();
        for index in self.repo_cache.values() {
            if let Some(versions) = index.packages.get(&request.name) {
                for pkg in versions {
                    if let Some(req) = &request.version_req {
                        if !req.matches(&pkg.version) { continue; }
                    }
                    out.push(pkg.clone());
                }
            }
        }
        Ok(out)
    }

    fn check_conflicts(&self, to_install: &[PackageMetadata], to_upgrade: &[(PackageMetadata, PackageMetadata)]) -> Result<()> {
        let all_new: Vec<&PackageMetadata> = to_install.iter().chain(to_upgrade.iter().map(|(_, n)| n)).collect();
        for pkg in &all_new {
            for conflict in &pkg.conflicts {
                for other in &all_new {
                    if other.name == conflict.name {
                        if conflict.version_req.is_none() {
                            return Err(anyhow!("Conflict: {} conflicts with {}", pkg.name, other.name));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
