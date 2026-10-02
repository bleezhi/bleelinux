use super::*;
use std::io::{Read, Write, Seek, SeekFrom};
use std::fs::File;
use std::path::Path;
use tar::Archive;
use zstd::stream::{Encoder, Decoder};
use blake3;
use sha2::{Sha256, Digest};

pub const PACKAGE_MAGIC: &[u8] = b"BLEEPKG\x01";
pub const METADATA_FILENAME: &str = ".METADATA.toml";
pub const FILES_FILENAME: &str = ".FILES.toml";
pub const SIGNATURE_FILENAME: &str = ".SIGNATURE.minisig";

#[derive(Debug, Clone)]
pub struct PackageArchive {
    pub metadata: PackageMetadata,
    pub files: Vec<FileEntry>,
    pub payload_offset: u64,
    pub payload_size: u64,
}

impl PackageArchive {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(&path)?;
        let mut decoder = Decoder::new(file)?;
        let mut magic = [0u8; 8];
        decoder.read_exact(&mut magic)?;
        if magic != PACKAGE_MAGIC {
            return Err(BleepkgError::Invalid("Not a valid bleepkg archive".into()));
        }

        let mut archive = Archive::new(&mut decoder);
        
        let mut metadata: Option<PackageMetadata> = None;
        let mut files: Option<Vec<FileEntry>> = None;
        let mut payload_offset = 0;
        
        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.to_path_buf();
            
            if path == Path::new(METADATA_FILENAME) {
                let mut content = String::new();
                entry.read_to_string(&mut content)?;
                metadata = Some(toml::from_str(&content)?);
            } else if path == Path::new(FILES_FILENAME) {
                let mut content = String::new();
                entry.read_to_string(&mut content)?;
                let list: FileList = toml::from_str(&content)?;
                files = Some(list.files);
            } else if path == Path::new(SIGNATURE_FILENAME) {
                // Skip signature in archive, handled separately
            } else {
                // Payload files
            }
            payload_offset += entry.size();
        }

        let metadata = metadata.ok_or_else(|| BleepkgError::Invalid("Missing metadata".into()))?;
        let files = files.unwrap_or_default();

        Ok(Self {
            metadata,
            files,
            payload_offset,
            payload_size: 0, // Will be calculated on extract
        })
    }

    pub fn extract<P: AsRef<Path>>(&self, _dest: P, _verify: bool) -> Result<()> {
        let dest = _dest.as_ref();
        std::fs::create_dir_all(dest)?;
        
        // Extract would be implemented here
        // For now, this is a placeholder
        Ok(())
    }

    pub fn verify_checksums<P: AsRef<Path>>(&self, path: P) -> Result<bool> {
        let file = File::open(path)?;
        let mut decoder = Decoder::new(file)?;
        
        // Skip magic
        let mut magic = [0u8; 8];
        decoder.read_exact(&mut magic)?;
        
        let mut hasher_sha256 = Sha256::new();
        let mut hasher_blake3 = blake3::Hasher::new();
        
        let mut buffer = vec![0u8; 65536];
        loop {
            let n = decoder.read(&mut buffer)?;
            if n == 0 { break; }
            hasher_sha256.update(&buffer[..n]);
            hasher_blake3.update(&buffer[..n]);
        }
        
        let sha256 = format!("{:x}", hasher_sha256.finalize());
        let blake3 = hasher_blake3.finalize().to_hex().to_string();
        
        Ok(sha256 == self.metadata.checksum_sha256 && blake3 == self.metadata.checksum_blake3)
    }
}

pub struct PackageBuilder {
    metadata: PackageMetadata,
    staging_dir: PathBuf,
    files: Vec<FileEntry>,
}

impl PackageBuilder {
    pub fn new(name: impl Into<String>, version: Version, arch: Architecture, libc: Libc) -> Self {
        let now = Utc::now();
        Self {
            metadata: PackageMetadata {
                id: Uuid::new_v4(),
                name: name.into(),
                version,
                release: 1,
                epoch: 0,
                description: String::new(),
                summary: String::new(),
                license: String::new(),
                url: String::new(),
                maintainer: String::new(),
                arch,
                libc,
                pkg_type: PackageType::Binary,
                build_time: now,
                build_host: whoami::fallible::hostname().unwrap_or_else(|_| "buildhost".to_string()),
                build_user: whoami::fallible::username().unwrap_or_else(|_| "builder".to_string()),
                source_url: None,
                git_commit: None,
                git_url: None,
                dependencies: vec![],
                provides: vec![],
                conflicts: vec![],
                replaces: vec![],
                files: vec![],
                installed_size: 0,
                download_size: 0,
                checksum_sha256: String::new(),
                checksum_blake3: String::new(),
                signature: None,
                custom_data: HashMap::new(),
            },
            staging_dir: tempfile::tempdir().unwrap().keep(),
            files: vec![],
        }
    }

    pub fn metadata_mut(&mut self) -> &mut PackageMetadata {
        &mut self.metadata
    }

    pub fn add_file<P: AsRef<Path>>(&mut self, src: P, dest: P, mode: u32) -> Result<()> {
        let src = src.as_ref();
        let dest = dest.as_ref();
        let dest_path = self.staging_dir.join(dest.strip_prefix("/").unwrap_or(dest));
        
        std::fs::create_dir_all(dest_path.parent().unwrap())?;
        std::fs::copy(src, &dest_path)?;
        
        let metadata = std::fs::metadata(&dest_path)?;
        let mut hasher_sha256 = Sha256::new();
        let mut hasher_blake3 = blake3::Hasher::new();
        let mut file = File::open(&dest_path)?;
        std::io::copy(&mut file, &mut hasher_sha256)?;
        std::io::copy(&mut File::open(&dest_path)?, &mut hasher_blake3)?;
        
        self.files.push(FileEntry {
            path: dest.to_path_buf(),
            size: metadata.len(),
            mode,
            uid: 0,
            gid: 0,
            mtime: DateTime::from(metadata.modified()?),
            sha256: format!("{:x}", hasher_sha256.finalize()),
            blake3: hasher_blake3.finalize().to_hex().to_string(),
            is_symlink: false,
            symlink_target: None,
            is_config: false,
            is_ghost: false,
        });
        
        Ok(())
    }

    pub fn add_symlink(&mut self, target: PathBuf, link: PathBuf) -> Result<()> {
        let link_path = self.staging_dir.join(link.strip_prefix("/").unwrap_or(&link));
        std::fs::create_dir_all(link_path.parent().unwrap())?;
        std::os::unix::fs::symlink(&target, &link_path)?;
        
        self.files.push(FileEntry {
            path: link,
            size: 0,
            mode: 0o777,
            uid: 0,
            gid: 0,
            mtime: Utc::now(),
            sha256: String::new(),
            blake3: String::new(),
            is_symlink: true,
            symlink_target: Some(target),
            is_config: false,
            is_ghost: false,
        });
        
        Ok(())
    }

    pub fn build<P: AsRef<Path>>(mut self, output: P) -> Result<PackageMetadata> {
        let output = output.as_ref();
        
        // Write metadata files
        let meta_path = self.staging_dir.join(METADATA_FILENAME);
        std::fs::write(&meta_path, toml::to_string_pretty(&self.metadata)?)?;
        
        let files_path = self.staging_dir.join(FILES_FILENAME);
        std::fs::write(&files_path, toml::to_string_pretty(&FileList { files: self.files.clone() })?)?;
        
        // Create archive
        let file = File::create(output)?;
        let mut encoder = Encoder::new(file, 19)?;  // Level 19 for best compression
        encoder.write_all(PACKAGE_MAGIC)?;
        {
            let mut tar = tar::Builder::new(&mut encoder);
            tar.append_dir_all(".", &self.staging_dir)?;
            tar.into_inner()?;
        }
        encoder.finish()?;
        
        // Calculate checksums of final archive
        let mut file = File::open(output)?;
        let mut hasher_sha256 = Sha256::new();
        let mut hasher_blake3 = blake3::Hasher::new();
        let mut buffer = vec![0u8; 65536];
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 { break; }
            hasher_sha256.update(&buffer[..n]);
            hasher_blake3.update(&buffer[..n]);
        }
        
        self.metadata.checksum_sha256 = format!("{:x}", hasher_sha256.finalize());
        self.metadata.checksum_blake3 = hasher_blake3.finalize().to_hex().to_string();
        self.metadata.files = self.files.clone();
        self.metadata.installed_size = self.files.iter().map(|f| f.size).sum();
        self.metadata.download_size = std::fs::metadata(output)?.len();
        
        // Rewrite metadata in archive with correct checksums
        // For simplicity, we'll just return the metadata
        // In production, we'd rewrite the archive
        
        Ok(self.metadata)
    }
}