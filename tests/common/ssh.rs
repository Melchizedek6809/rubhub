use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;

/// SSH key type for test key generation
#[derive(Debug, Clone, Copy)]
pub enum KeyType {
    Ed25519,
    Rsa,
}

impl KeyType {
    fn as_str(&self) -> &'static str {
        match self {
            KeyType::Ed25519 => "ed25519",
            KeyType::Rsa => "rsa",
        }
    }
}

/// Represents an SSH keypair for testing
pub struct TestSshKey {
    pub private_key_path: PathBuf,
    pub public_key_path: PathBuf,
    pub public_key_content: String,
    pub key_type: KeyType,
}

impl TestSshKey {
    /// Generate a new SSH keypair in the given directory.
    ///
    /// Key files are named `{name}_{key_type}` to avoid overwriting
    /// the server's host keys (`id_ed25519`, `id_rsa`).
    pub fn generate(dir: &Path, name: &str, key_type: KeyType) -> Result<Self> {
        let key_suffix = key_type.as_str();
        let private_key_path = dir.join(format!("{}_{}", name, key_suffix));
        let public_key_path = dir.join(format!("{}_{}.pub", name, key_suffix));

        // Remove existing key files if they exist (ssh-keygen won't overwrite)
        let _ = std::fs::remove_file(&private_key_path);
        let _ = std::fs::remove_file(&public_key_path);

        let output = Command::new("ssh-keygen")
            .arg("-t")
            .arg(key_suffix)
            .arg("-N")
            .arg("") // No passphrase
            .arg("-C")
            .arg(format!("{}@test", name))
            .arg("-f")
            .arg(&private_key_path)
            .output()?;

        if !output.status.success() {
            anyhow::bail!(
                "ssh-keygen failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let public_key_content = std::fs::read_to_string(&public_key_path)?;

        Ok(Self {
            private_key_path,
            public_key_path,
            public_key_content: public_key_content.trim().to_string(),
            key_type,
        })
    }

    /// Generate a new Ed25519 SSH keypair (convenience method)
    pub fn generate_ed25519(dir: &Path, name: &str) -> Result<Self> {
        Self::generate(dir, name, KeyType::Ed25519)
    }

    /// Generate a new RSA SSH keypair (convenience method)
    pub fn generate_rsa(dir: &Path, name: &str) -> Result<Self> {
        Self::generate(dir, name, KeyType::Rsa)
    }
}
