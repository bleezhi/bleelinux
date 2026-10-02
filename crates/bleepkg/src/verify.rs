use super::*;
use std::path::Path;
use std::fs::File;
use std::io::Read;
use tar::Archive;
use zstd::stream::Decoder;
use sha2::{Sha256, Digest};

pub struct Verifier {
    pub trusted_keys: Vec<String>,  // Minisign public keys
    pub require_signature: bool,
}

impl Verifier {
    pub fn new(trusted_keys: Vec<String>, require_signature: bool) -> Self {
        Self { trusted_keys, require_signature }
    }

    pub fn verify_package<P: AsRef<Path>>(&self, pkg_path: P) -> Result<VerificationResult> {
        let pkg_path = pkg_path.as_ref();
        
        // Open and read package
        let file = File::open(pkg_path)?;
        let mut decoder = Decoder::new(file)?;
        
        // Verify magic
        let mut magic = [0u8; 8];
        decoder.read_exact(&mut magic)?;
        if magic != PACKAGE_MAGIC {
            return Err(BleepkgError::Invalid("Invalid package magic".into()));
        }

        // Calculate checksums of compressed payload
        let mut hasher_sha256 = Sha256::new();
        let mut hasher_blake3 = blake3::Hasher::new();
        let mut buffer = vec![0u8; 65536];
        
        loop {
            let n = decoder.read(&mut buffer)?;
            if n == 0 { break; }
            hasher_sha256.update(&buffer[..n]);
            hasher_blake3.update(&buffer[..n]);
        }

        let computed_sha256 = format!("{:x}", hasher_sha256.finalize());
        let computed_blake3 = hasher_blake3.finalize().to_hex().to_string();

        // Re-open to read metadata
        let file = File::open(pkg_path)?;
        let mut decoder = Decoder::new(file)?;
        decoder.read_exact(&mut magic)?; // Skip magic
        
        let mut archive = Archive::new(&mut decoder);
        let mut metadata: Option<PackageMetadata> = None;
        let mut signature: Option<String> = None;

        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.to_path_buf();
            
            if path == Path::new(METADATA_FILENAME) {
                let mut content = String::new();
                entry.read_to_string(&mut content)?;
                metadata = Some(toml::from_str(&content)?);
            } else if path == Path::new(SIGNATURE_FILENAME) {
                let mut content = String::new();
                entry.read_to_string(&mut content)?;
                signature = Some(content);
            }
        }

        let metadata = metadata.ok_or_else(|| BleepkgError::Invalid("Missing metadata".into()))?;

        // Verify checksums match metadata
        let sha256_ok = computed_sha256 == metadata.checksum_sha256;
        let blake3_ok = computed_blake3 == metadata.checksum_blake3;

        // Verify signature if present
        let sig_ok = if let Some(sig) = &signature {
            self.verify_signature(pkg_path, sig)?
        } else if self.require_signature {
            return Ok(VerificationResult {
                sha256_ok,
                blake3_ok,
                signature_ok: false,
                trusted: false,
                errors: vec!["Package not signed but signature required".into()],
            });
        } else {
            true
        };

        let trusted = sig_ok && self.is_trusted_signer(&metadata)?;

        Ok(VerificationResult {
            sha256_ok,
            blake3_ok,
            signature_ok: sig_ok,
            trusted,
            errors: vec![],
        })
    }

    fn verify_signature<P: AsRef<Path>>(&self, pkg_path: P, signature: &str) -> Result<bool> {
        // Verify minisign signature
        // This is a simplified version - real implementation would use minisign crate
        for key in &self.trusted_keys {
            // minisign::verify_detached(pkg_path, signature, key)?
        }
        Ok(true)  // Placeholder
    }

    fn is_trusted_signer(&self, metadata: &PackageMetadata) -> Result<bool> {
        // Check if the package's build key is in trusted keys
        // This would be stored in metadata.custom_data["signing_key"]
        Ok(true)  // Placeholder
    }

    pub fn verify_installed(&self, db: &PackageDatabase, name: &str) -> Result<VerificationResult> {
        let pkg = db.get_installed(name)?
            .ok_or_else(|| BleepkgError::Invalid(format!("Package {name} not installed")))?;

        let mut errors = Vec::new();
        let mut all_ok = true;

        for file_entry in &pkg.files {
            if file_entry.is_ghost || file_entry.is_symlink {
                continue;
            }
            let path = Path::new("/").join(file_entry.path.strip_prefix("/").unwrap_or(&file_entry.path));
            
            if !path.exists() {
                errors.push(format!("Missing file: {}", file_entry.path.display()));
                all_ok = false;
                continue;
            }

            let mut f = File::open(&path)?;
            let mut hasher_sha256 = Sha256::new();
            let mut hasher_blake3 = blake3::Hasher::new();
            let mut buffer = vec![0u8; 65536];
            
            loop {
                let n = f.read(&mut buffer)?;
                if n == 0 { break; }
                hasher_sha256.update(&buffer[..n]);
                hasher_blake3.update(&buffer[..n]);
            }

            let sha256 = format!("{:x}", hasher_sha256.finalize());
            let blake3 = hasher_blake3.finalize().to_hex().to_string();

            if sha256 != file_entry.sha256 {
                errors.push(format!("SHA256 mismatch: {}", file_entry.path.display()));
                all_ok = false;
            }
            if blake3 != file_entry.blake3 {
                errors.push(format!("BLAKE3 mismatch: {}", file_entry.path.display()));
                all_ok = false;
            }
        }

        Ok(VerificationResult {
            sha256_ok: all_ok,
            blake3_ok: all_ok,
            signature_ok: true,
            trusted: all_ok,
            errors,
        })
    }
}

#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub sha256_ok: bool,
    pub blake3_ok: bool,
    pub signature_ok: bool,
    pub trusted: bool,
    pub errors: Vec<String>,
}

impl VerificationResult {
    pub fn is_valid(&self) -> bool {
        self.sha256_ok && self.blake3_ok && self.signature_ok && self.trusted
    }
}