-- Package database schema
CREATE TABLE IF NOT EXISTS packages (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    version TEXT NOT NULL,
    release INTEGER NOT NULL,
    epoch INTEGER NOT NULL DEFAULT 0,
    arch TEXT NOT NULL,
    libc TEXT NOT NULL,
    metadata TEXT NOT NULL,  -- Full JSON metadata
    installed_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_packages_name ON packages(name);
CREATE INDEX IF NOT EXISTS idx_packages_arch_libc ON packages(arch, libc);

CREATE TABLE IF NOT EXISTS files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    package_id TEXT NOT NULL,
    path TEXT NOT NULL,
    size INTEGER NOT NULL,
    mode INTEGER NOT NULL,
    sha256 TEXT NOT NULL,
    blake3 TEXT NOT NULL,
    is_config BOOLEAN NOT NULL DEFAULT 0,
    is_ghost BOOLEAN NOT NULL DEFAULT 0,
    FOREIGN KEY (package_id) REFERENCES packages(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_files_package ON files(package_id);
CREATE INDEX IF NOT EXISTS idx_files_path ON files(path);

CREATE TABLE IF NOT EXISTS dependencies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    package_id TEXT NOT NULL,
    dep_name TEXT NOT NULL,
    version_req TEXT NOT NULL,
    optional BOOLEAN NOT NULL DEFAULT 0,
    FOREIGN KEY (package_id) REFERENCES packages(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_deps_package ON dependencies(package_id);
CREATE INDEX IF NOT EXISTS idx_deps_name ON dependencies(dep_name);

CREATE TABLE IF NOT EXISTS repo_index (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    repo_name TEXT NOT NULL,
    repo_url TEXT NOT NULL,
    arch TEXT NOT NULL,
    libc TEXT NOT NULL,
    package_name TEXT NOT NULL,
    package_version TEXT NOT NULL,
    package_release INTEGER NOT NULL,
    package_epoch INTEGER NOT NULL DEFAULT 0,
    download_url TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    blake3 TEXT NOT NULL,
    size INTEGER NOT NULL,
    installed_size INTEGER NOT NULL,
    dependencies TEXT NOT NULL,  -- JSON array
    provides TEXT NOT NULL,      -- JSON array
    conflicts TEXT NOT NULL,     -- JSON array
    fetched_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_repo_pkg 
ON repo_index(repo_name, package_name, package_version, package_release, package_epoch, arch, libc);

CREATE TABLE IF NOT EXISTS transactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    action TEXT NOT NULL,  -- install, upgrade, remove
    packages TEXT NOT NULL,  -- JSON array
    success BOOLEAN NOT NULL,
    log TEXT
);

CREATE TABLE IF NOT EXISTS config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Initial config
INSERT OR IGNORE INTO config (key, value) VALUES ('db_version', '1');