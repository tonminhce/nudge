use std::path::Path;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::file::storage::StorageError;
use crate::file::storage::StoredFile;

#[cfg(unix)]
fn stat_checksum(metadata: &std::fs::Metadata) -> String {
    use std::os::unix::fs::MetadataExt;

    format!(
        "{}-{}-{}",
        metadata.ino(),
        metadata.len(),
        modified_milliseconds(metadata)
    )
}

#[cfg(not(unix))]
fn stat_checksum(metadata: &std::fs::Metadata) -> String {
    format!("{}-{}", metadata.len(), modified_milliseconds(metadata))
}

fn modified_milliseconds(metadata: &std::fs::Metadata) -> u128 {
    metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|elapsed| elapsed.as_millis())
        .unwrap_or_default()
}

#[derive(Debug, Clone)]
pub struct LocalDriver {
    root: PathBuf,
}

impl LocalDriver {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    fn canonical_root(&self) -> Result<PathBuf, StorageError> {
        std::fs::canonicalize(&self.root).map_err(|error| StorageError::Backend(error.to_string()))
    }

    fn assert_within_storage(&self, resolved: &Path) -> Result<PathBuf, StorageError> {
        let root = self.canonical_root()?;

        if !resolved.starts_with(&root) {
            return Err(StorageError::AccessDenied);
        }

        Ok(resolved.to_path_buf())
    }

    fn resolve_readable(&self, file_path: &str) -> Result<PathBuf, StorageError> {
        let joined = self.root.join(file_path);
        let resolved = std::fs::canonicalize(&joined).map_err(|_| StorageError::FileNotFound)?;

        self.assert_within_storage(&resolved)
    }

    fn resolve_writable(&self, file_path: &str) -> Result<PathBuf, StorageError> {
        let joined = absolute(&self.root, file_path);
        let folder = joined.parent().unwrap_or(Path::new("/"));

        std::fs::create_dir_all(folder)
            .map_err(|error| StorageError::Backend(error.to_string()))?;

        let real_folder = std::fs::canonicalize(folder)
            .map_err(|error| StorageError::Backend(error.to_string()))?;
        let target = real_folder.join(joined.file_name().ok_or(StorageError::InvalidPath(
            String::from("Invalid storage path: missing file name"),
        ))?);

        self.assert_within_storage(&target)?;

        if std::fs::symlink_metadata(&target)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false)
        {
            return Err(StorageError::AccessDenied);
        }

        Ok(target)
    }

    pub fn start_write(&self, file_path: &str) -> Result<LocalWriter, StorageError> {
        let target = self.resolve_writable(file_path)?;
        let partial = target
            .parent()
            .unwrap_or(Path::new("/"))
            .join(format!(".{}.part", Uuid::new_v4()));
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&partial)
            .map_err(|error| StorageError::Backend(error.to_string()))?;

        Ok(LocalWriter {
            file: tokio::fs::File::from_std(file),
            partial,
            target,
        })
    }

    pub async fn metadata(&self, file_path: &str) -> Result<Option<StoredFile>, StorageError> {
        let resolved = match self.resolve_readable(file_path) {
            Ok(resolved) => resolved,
            Err(StorageError::FileNotFound) => return Ok(None),
            Err(error) => return Err(error),
        };
        let metadata = tokio::fs::metadata(&resolved)
            .await
            .map_err(|error| StorageError::Backend(error.to_string()))?;

        if !metadata.is_file() {
            return Ok(None);
        }

        Ok(Some(StoredFile {
            size: metadata.len(),
            checksum: Some(stat_checksum(&metadata)),
        }))
    }

    pub async fn read_prefix(
        &self,
        file_path: &str,
        byte_count: u64,
    ) -> Result<Vec<u8>, StorageError> {
        use tokio::io::AsyncReadExt;

        let resolved = self.resolve_readable(file_path)?;
        let file = tokio::fs::File::open(&resolved)
            .await
            .map_err(|_| StorageError::FileNotFound)?;
        let length = usize::try_from(byte_count).unwrap_or(usize::MAX);
        let mut buffer = Vec::with_capacity(length.min(1024 * 1024));

        file.take(byte_count)
            .read_to_end(&mut buffer)
            .await
            .map_err(|error| StorageError::Backend(error.to_string()))?;

        Ok(buffer)
    }

    pub async fn rename(
        &self,
        from: &str,
        to: &str,
        if_match_checksum: Option<&str>,
    ) -> Result<(), StorageError> {
        let source = absolute(&self.root, from);
        let target = absolute(&self.root, to);
        let folder = target.parent().unwrap_or(Path::new("/"));

        std::fs::create_dir_all(folder)
            .map_err(|error| StorageError::Backend(error.to_string()))?;

        match if_match_checksum {
            Some(checksum) => self.rename_preconditioned(&source, &target, checksum).await,
            None => tokio::fs::rename(&source, &target)
                .await
                .map_err(|error| rename_error(&error)),
        }
    }

    async fn rename_preconditioned(
        &self,
        source: &Path,
        target: &Path,
        expected_checksum: &str,
    ) -> Result<(), StorageError> {
        let claim = source
            .parent()
            .unwrap_or(Path::new("/"))
            .join(format!(".{}.promoting", Uuid::new_v4()));

        tokio::fs::rename(source, &claim)
            .await
            .map_err(|error| rename_error(&error))?;

        let metadata = tokio::fs::metadata(&claim)
            .await
            .map_err(|error| rename_error(&error))?;

        if stat_checksum(&metadata) != expected_checksum {
            let _ = tokio::fs::rename(&claim, source).await;

            return Err(StorageError::PreconditionFailed(format!(
                "Object {} changed since it was inspected",
                target
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default()
            )));
        }

        if let Err(error) = tokio::fs::rename(&claim, target).await {
            let _ = tokio::fs::rename(&claim, source).await;

            return Err(rename_error(&error));
        }

        Ok(())
    }

    pub async fn delete(&self, file_path: &str) -> Result<(), StorageError> {
        let root = self.canonical_root()?;
        let resolved = absolute(&root, file_path);

        if !resolved.starts_with(&root) {
            return Err(StorageError::AccessDenied);
        }

        match tokio::fs::remove_file(&resolved).await {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                return Err(StorageError::Backend(error.to_string()));
            }
            Err(_) => {}
        }

        match tokio::fs::remove_dir_all(&resolved).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(StorageError::Backend(error.to_string())),
        }
    }
}

pub struct LocalWriter {
    file: tokio::fs::File,
    partial: PathBuf,
    target: PathBuf,
}

impl LocalWriter {
    pub async fn write(&mut self, chunk: &[u8]) -> Result<(), StorageError> {
        self.file
            .write_all(chunk)
            .await
            .map_err(|error| StorageError::Backend(error.to_string()))
    }

    pub async fn finish(mut self) -> Result<(), StorageError> {
        self.file
            .flush()
            .await
            .map_err(|error| StorageError::Backend(error.to_string()))?;
        self.file
            .shutdown()
            .await
            .map_err(|error| StorageError::Backend(error.to_string()))?;

        tokio::fs::rename(&self.partial, &self.target)
            .await
            .map_err(|error| StorageError::Backend(error.to_string()))
    }

    pub async fn abort(self) {
        let _ = tokio::fs::remove_file(&self.partial).await;
    }
}

fn absolute(root: &Path, file_path: &str) -> PathBuf {
    let mut resolved = root.to_path_buf();

    for segment in file_path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                resolved.pop();
            }
            value => resolved.push(value),
        }
    }

    resolved
}

fn rename_error(error: &std::io::Error) -> StorageError {
    if error.kind() == std::io::ErrorKind::NotFound {
        return StorageError::FileNotFound;
    }

    StorageError::Backend(error.to_string())
}
