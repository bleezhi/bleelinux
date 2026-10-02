use super::*;
use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use semver::Version;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Architecture {
    #[serde(alias = "x86_64", alias = "amd64")]
    X86_64,
    #[serde(alias = "arm64")]
    Aarch64,
    Riscv64,
    Ppc64le,
    // Add more as needed
    Other(String),
}

impl FromStr for Architecture {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "x86_64" | "amd64" => Architecture::X86_64,
            "aarch64" | "arm64" => Architecture::Aarch64,
            "riscv64" => Architecture::Riscv64,
            "ppc64le" => Architecture::Ppc64le,
            other => Architecture::Other(other.to_string()),
        })
    }
}

impl std::fmt::Display for Architecture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Architecture::X86_64 => write!(f, "x86_64"),
            Architecture::Aarch64 => write!(f, "aarch64"),
            Architecture::Riscv64 => write!(f, "riscv64"),
            Architecture::Ppc64le => write!(f, "ppc64le"),
            Architecture::Other(s) => write!(f, "{s}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Libc {
    Glibc,
    Musl,
    Other(String),
}

impl FromStr for Libc {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "glibc" => Libc::Glibc,
            "musl" => Libc::Musl,
            other => Libc::Other(other.to_string()),
        })
    }
}

impl std::fmt::Display for Libc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Libc::Glibc => write!(f, "glibc"),
            Libc::Musl => write!(f, "musl"),
            Libc::Other(s) => write!(f, "{s}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageType {
    Binary,
    Source,
    Metapackage,
    Firmware,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version_req: String,  // semver requirement (e.g., ">=1.0.0,<2.0.0")
    pub arch: Option<Architecture>,
    pub libc: Option<Libc>,
    pub optional: bool,
}

impl Dependency {
    pub fn new(name: impl Into<String>, version_req: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version_req: version_req.into(),
            arch: None,
            libc: None,
            optional: false,
        }
    }

    pub fn matches(&self, pkg: &PackageMetadata) -> bool {
        if self.name != pkg.name {
            return false;
        }
        let req = semver::VersionReq::parse(&self.version_req).ok();
        req.map(|r| r.matches(&pkg.version)).unwrap_or(false)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provides {
    pub name: String,
    pub version: Version,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflicts {
    pub name: String,
    pub version_req: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replaces {
    pub name: String,
    pub version_req: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileList {
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub mtime: DateTime<Utc>,
    pub sha256: String,
    pub blake3: String,
    pub is_symlink: bool,
    pub symlink_target: Option<PathBuf>,
    pub is_config: bool,
    pub is_ghost: bool,  // Not installed, tracked for removal
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    pub id: Uuid,
    pub name: String,
    pub version: Version,
    pub release: u32,
    pub epoch: u32,
    pub description: String,
    pub summary: String,
    pub license: String,
    pub url: String,
    pub maintainer: String,
    pub arch: Architecture,
    pub libc: Libc,
    pub pkg_type: PackageType,
    pub build_time: DateTime<Utc>,
    pub build_host: String,
    pub build_user: String,
    pub source_url: Option<String>,
    pub git_commit: Option<String>,
    pub git_url: Option<String>,
    pub dependencies: Vec<Dependency>,
    pub provides: Vec<Provides>,
    pub conflicts: Vec<Conflicts>,
    pub replaces: Vec<Replaces>,
    pub files: Vec<FileEntry>,
    pub installed_size: u64,
    pub download_size: u64,
    pub checksum_sha256: String,
    pub checksum_blake3: String,
    pub signature: Option<String>,  // Minisign signature
    pub custom_data: HashMap<String, serde_json::Value>,
}

impl PackageMetadata {
    pub fn full_version(&self) -> String {
        if self.epoch > 0 {
            format!("{}:{}-{}", self.epoch, self.version, self.release)
        } else {
            format!("{}-{}", self.version, self.release)
        }
    }

    pub fn package_filename(&self) -> String {
        format!(
            "{}-{}-{}.{}.{}.bleepkg",
            self.name,
            self.full_version(),
            self.arch,
            self.libc,
            self.pkg_type_str()
        )
    }

    fn pkg_type_str(&self) -> &'static str {
        match self.pkg_type {
            PackageType::Binary => "bin",
            PackageType::Source => "src",
            PackageType::Metapackage => "meta",
            PackageType::Firmware => "fw",
        }
    }

    pub fn satisfies(&self, dep: &Dependency) -> bool {
        dep.matches(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageIndex {
    pub packages: HashMap<String, Vec<PackageMetadata>>,  // name -> versions
    pub generated: DateTime<Utc>,
    pub repo_name: String,
    pub repo_url: String,
    pub arch: Architecture,
    pub libc: Libc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildRecipe {
    pub pkg_name: String,
    pub version: Version,
    pub release: u32,
    pub epoch: u32,
    pub source: SourceSpec,
    pub dependencies: Vec<Dependency>,
    pub build_dependencies: Vec<Dependency>,
    pub configure_args: Vec<String>,
    pub make_args: Vec<String>,
    pub install_args: Vec<String>,
    pub patches: Vec<PatchSpec>,
    pub subpackages: Vec<SubpackageSpec>,
    pub options: BuildOptions,
    pub maintainer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SourceSpec {
    Url { url: String, sha256: String },
    Git { url: String, rev: String, submodules: bool },
    Local { path: PathBuf },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchSpec {
    pub url: Option<String>,
    pub path: Option<PathBuf>,
    pub sha256: Option<String>,
    pub apply_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubpackageSpec {
    pub name: String,
    pub description: String,
    pub files: Vec<String>,  // Glob patterns
    pub dependencies: Vec<Dependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildOptions {
    pub no_strip: bool,
    pub no_split_docs: bool,
    pub no_split_locale: bool,
    pub no_split_dev: bool,
    pub static_link: bool,
    pub pie: bool,
    pub relro: bool,
    pub fortify: bool,
    pub lto: bool,
    pub cross_compile: bool,
    pub custom_env: HashMap<String, String>,
}

impl PackageMetadata {
    pub fn from_recipe(recipe: &BuildRecipe, arch: Architecture, libc: Libc) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: recipe.pkg_name.clone(),
            version: recipe.version.clone(),
            release: recipe.release,
            epoch: recipe.epoch,
            description: String::new(),
            summary: String::new(),
            license: String::new(),
            url: String::new(),
            maintainer: recipe.maintainer.clone(),
            arch,
            libc,
            pkg_type: PackageType::Binary,
            build_time: now,
            build_host: whoami::fallible::hostname().unwrap_or_else(|_| "buildhost".to_string()),
            build_user: whoami::fallible::username().unwrap_or_else(|_| "builder".to_string()),
            source_url: None,
            git_commit: None,
            git_url: None,
            dependencies: recipe.dependencies.clone(),
            provides: vec![Provides {
                name: recipe.pkg_name.clone(),
                version: recipe.version.clone(),
            }],
            conflicts: vec![],
            replaces: vec![],
            files: vec![],
            installed_size: 0,
            download_size: 0,
            checksum_sha256: String::new(),
            checksum_blake3: String::new(),
            signature: None,
            custom_data: HashMap::new(),
        }
    }
}