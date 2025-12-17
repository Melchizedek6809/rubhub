use anyhow::Result;
use std::path::Path;
use uuid::Uuid;

/// Atomically writes data to a file by first writing to a temporary file
/// and then renaming it to the target path.
///
/// This provides better safety in concurrent contexts by ensuring that
/// readers never see a partially written file.
///
/// # Arguments
///
/// * `path` - The target file path
/// * `data` - The data to write
pub async fn atomic_write<P: AsRef<Path>>(path: P, data: impl AsRef<[u8]>) -> Result<()> {
    let path = path.as_ref();

    // Generate temporary filename: UUID + original filename as suffix
    let temp_filename = if let Some(filename) = path.file_name() {
        format!(".tmp-{}-{}", Uuid::new_v4(), filename.to_string_lossy())
    } else {
        format!(".tmp-{}", Uuid::new_v4())
    };

    // Place temp file in same directory as target to ensure same filesystem
    let temp_path = if let Some(parent) = path.parent() {
        parent.join(temp_filename)
    } else {
        temp_filename.into()
    };

    // Write to temporary file
    tokio::fs::write(&temp_path, data).await?;

    // Atomically rename to target path (atomic on same filesystem)
    tokio::fs::rename(&temp_path, path).await?;

    Ok(())
}
