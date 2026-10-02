use crate::{PackageManager, Config};
use std::path::PathBuf;
use clap::{Parser, Subcommand, Args};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "bleepm", version, about = "Bleelinux Package Manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    #[arg(short, long, global = true)]
    pub root: Option<PathBuf>,

    #[arg(short = 'y', long, global = true)]
    pub yes: bool,

    #[arg(long, global = true)]
    pub dry_run: bool,

    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Install packages
    Install(InstallArgs),
    /// Remove packages
    Remove(RemoveArgs),
    /// Upgrade packages
    Upgrade(UpgradeArgs),
    /// Search packages
    Search(SearchArgs),
    /// Show package info
    Info(InfoArgs),
    /// List installed packages
    List(ListArgs),
    /// Build from source
    Build(BuildArgs),
    /// Manage repositories
    Repo(RepoArgs),
    /// Verify installed packages
    Verify(VerifyArgs),
    /// Clean cache
    Clean(CleanArgs),
    /// Pack a directory tree into a .bleepkg
    Pack(PackArgs),
    /// Build index.json.zst from .bleepkg files in a directory
    RepoIndex(RepoIndexArgs),
    /// Show transaction history
    History,
}

#[derive(Args)]
pub struct InstallArgs {
    /// Package names or specifications (pkg, pkg=1.0, pkg@git)
    pub packages: Vec<String>,
    
    /// Build from git source instead of binary
    #[arg(long)]
    pub from_git: bool,
    
    /// Git URL for source build
    #[arg(long)]
    pub git_url: Option<String>,
    
    /// Skip dependency resolution
    #[arg(long)]
    pub no_deps: bool,
}

#[derive(Args)]
pub struct RemoveArgs {
    pub packages: Vec<String>,
    
    /// Also remove dependencies that are no longer needed
    #[arg(long)]
    pub recursive: bool,
    
    /// Remove config files too
    #[arg(long)]
    pub purge: bool,
}

#[derive(Args)]
pub struct UpgradeArgs {
    /// Specific packages to upgrade (default: all)
    pub packages: Vec<String>,
    
    /// Upgrade to latest including breaking changes
    #[arg(long)]
    pub full: bool,
}

#[derive(Args)]
pub struct SearchArgs {
    pub query: String,
    
    /// Search in descriptions too
    #[arg(short, long)]
    pub all: bool,
    
    /// Show only installed
    #[arg(long)]
    pub installed: bool,
}

#[derive(Args)]
pub struct InfoArgs {
    pub package: String,
    
    /// Show files
    #[arg(long)]
    pub files: bool,
    
    /// Show dependencies
    #[arg(long)]
    pub deps: bool,
}

#[derive(Args)]
pub struct ListArgs {
    /// Show only explicitly installed
    #[arg(long)]
    pub explicit: bool,
    
    /// Show size
    #[arg(long)]
    pub size: bool,
}

#[derive(Args)]
pub struct BuildArgs {
    pub package: String,
    
    /// Git URL to build from
    #[arg(long)]
    pub git: Option<String>,
    
    /// Git revision/commit
    #[arg(long)]
    pub rev: Option<String>,
    
    /// Install after building
    #[arg(long)]
    pub install: bool,
}

#[derive(Args)]
pub struct RepoArgs {
    #[command(subcommand)]
    pub action: RepoAction,
}

#[derive(Subcommand)]
pub enum RepoAction {
    /// List configured repositories
    List,
    /// Add a repository
    Add { name: String, url: String },
    /// Remove a repository
    Remove { name: String },
    /// Enable a repository
    Enable { name: String },
    /// Disable a repository
    Disable { name: String },
    /// Refresh all repositories
    Refresh,
}

#[derive(Args)]
pub struct VerifyArgs {
    /// Package to verify (default: all)
    pub package: Option<String>,
    
    /// Fix mismatches
    #[arg(long)]
    pub fix: bool,
}

#[derive(Args)]
pub struct CleanArgs {
    /// Clean downloaded packages
    #[arg(long)]
    pub packages: bool,

    /// Clean build directories
    #[arg(long)]
    pub build: bool,

    /// Clean everything
    #[arg(long)]
    pub all: bool,
}

#[derive(Args)]
pub struct PackArgs {
    /// Directory tree to pack (files land at /<relpath>)
    pub dir: PathBuf,
    /// Package name
    #[arg(long)]
    pub name: String,
    /// Package version (semver)
    #[arg(long)]
    pub version: String,
    /// Output .bleepkg path
    #[arg(short, long)]
    pub output: PathBuf,
}

#[derive(Args)]
pub struct RepoIndexArgs {
    /// Directory containing .bleepkg files
    pub dir: PathBuf,
}

pub async fn run(cli: Cli) -> Result<()> {
    let mut config = if let Some(path) = cli.config.clone() {
        Config::load_from(path)?
    } else {
        Config::load()?
    };

    if let Some(root) = cli.root {
        config.root_dir = root;
    }

    let mut pm = PackageManager::new(config)?;
    if let Some(path) = cli.config {
        pm = pm.with_config_path(path);
    }
    
    match cli.command {
        Commands::Install(args) => {
            pm.refresh_repos().await?;
            let mut specs = args.packages;
            if args.from_git {
                specs = specs.iter().map(|s| format!("{}@git", s)).collect();
            }
            pm.install(&specs, cli.dry_run, cli.yes).await?;
        }
        Commands::Remove(args) => {
            pm.remove(&args.packages, args.recursive, args.purge, cli.dry_run).await?;
        }
        Commands::Upgrade(args) => {
            pm.refresh_repos().await?;
            pm.upgrade(&args.packages, args.full, cli.dry_run).await?;
        }
        Commands::Search(args) => {
            pm.refresh_repos().await?;
            let results = pm.search(&args.query).await?;
            for pkg in results {
                println!("{} {} - {}", pkg.name, pkg.full_version(), pkg.summary);
            }
        }
        Commands::Info(args) => {
            pm.refresh_repos().await?;
            pm.show_info(&args.package, args.files, args.deps).await?;
        }
        Commands::List(args) => {
            pm.list_installed(args.explicit, args.size).await?;
        }
        Commands::Build(args) => {
            let pkg = pm.build_from_source(&args.package, args.git).await?;
            println!("Built: {}", pkg.package_filename());
            if args.install {
                pm.install(&[pkg.name], false, cli.yes).await?;
            }
        }
        Commands::Repo(args) => {
            pm.repo_command(args.action).await?;
        }
        Commands::Verify(args) => {
            pm.verify(args.package.as_deref(), args.fix).await?;
        }
        Commands::Clean(args) => {
            pm.clean(args.packages, args.build, args.all).await?;
        }
        Commands::Pack(args) => {
            pm.pack(&args.dir, &args.output, &args.name, &args.version)?;
        }
        Commands::RepoIndex(args) => {
            pm.repo_index(&args.dir)?;
        }
        Commands::History => {
            pm.show_history().await?;
        }
    }
    
    Ok(())
}