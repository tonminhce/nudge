use std::fmt;

use uuid::Uuid;

use crate::file::drivers::local::LocalDriver;
use crate::file::drivers::local::LocalWriter;
use crate::file::drivers::s3::S3Driver;
use crate::file::folder::FileFolder;
use crate::file::validation::assert_storage_path_is_safe;
use crate::file::validation::validate_file_path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteWindow {
    pub start_byte: u64,
    pub end_byte: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredFile {
    pub size: u64,
    pub checksum: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    FileNotFound,
    AccessDenied,
    InvalidPath(String),
    PreconditionFailed(String),
    Backend(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileNotFound => formatter.write_str("File not found"),
            Self::AccessDenied => formatter.write_str("Access denied"),
            Self::InvalidPath(message) => formatter.write_str(message),
            Self::PreconditionFailed(message) => formatter.write_str(message),
            Self::Backend(message) => formatter.write_str(message),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileLocation {
    pub workspace_id: Uuid,
    pub application_universal_identifier: Uuid,
    pub folder: FileFolder,
}

#[derive(Debug, Clone)]
pub enum RawDriver {
    Local(LocalDriver),
    S3(S3Driver),
}

#[derive(Debug, Clone)]
pub struct ValidatedDriver {
    raw: RawDriver,
}

impl ValidatedDriver {
    pub fn new(raw: RawDriver) -> Self {
        Self { raw }
    }

    pub fn start_write(&self, file_path: &str) -> Result<Writer, StorageError> {
        assert_storage_path_is_safe(file_path).map_err(StorageError::InvalidPath)?;

        match &self.raw {
            RawDriver::Local(driver) => Ok(Writer::Local(driver.start_write(file_path)?)),
            RawDriver::S3(_) => Err(StorageError::Backend(S3_DATA_PLANE_MESSAGE.to_string())),
        }
    }

    pub async fn metadata(&self, file_path: &str) -> Result<Option<StoredFile>, StorageError> {
        assert_storage_path_is_safe(file_path).map_err(StorageError::InvalidPath)?;

        match &self.raw {
            RawDriver::Local(driver) => driver.metadata(file_path).await,
            RawDriver::S3(_) => Err(StorageError::Backend(S3_DATA_PLANE_MESSAGE.to_string())),
        }
    }

    pub async fn read_prefix(
        &self,
        file_path: &str,
        byte_count: u64,
    ) -> Result<Vec<u8>, StorageError> {
        assert_storage_path_is_safe(file_path).map_err(StorageError::InvalidPath)?;

        match &self.raw {
            RawDriver::Local(driver) => driver.read_prefix(file_path, byte_count).await,
            RawDriver::S3(_) => Err(StorageError::Backend(S3_DATA_PLANE_MESSAGE.to_string())),
        }
    }

    pub async fn rename(
        &self,
        from: &str,
        to: &str,
        if_match_checksum: Option<&str>,
    ) -> Result<(), StorageError> {
        assert_storage_path_is_safe(from).map_err(StorageError::InvalidPath)?;
        assert_storage_path_is_safe(to).map_err(StorageError::InvalidPath)?;

        match &self.raw {
            RawDriver::Local(driver) => driver.rename(from, to, if_match_checksum).await,
            RawDriver::S3(_) => Err(StorageError::Backend(S3_DATA_PLANE_MESSAGE.to_string())),
        }
    }

    pub async fn delete(&self, file_path: &str) -> Result<(), StorageError> {
        assert_storage_path_is_safe(file_path).map_err(StorageError::InvalidPath)?;

        match &self.raw {
            RawDriver::Local(driver) => driver.delete(file_path).await,
            RawDriver::S3(_) => Err(StorageError::Backend(S3_DATA_PLANE_MESSAGE.to_string())),
        }
    }

    pub fn presigned_upload_url(
        &self,
        file_path: &str,
        content_type: &str,
        content_length: u64,
        expires_in_secs: u64,
    ) -> Option<String> {
        assert_storage_path_is_safe(file_path).ok()?;

        match &self.raw {
            RawDriver::Local(_) => None,
            RawDriver::S3(driver) => Some(
                driver
                    .presigned_upload_url(file_path, content_type, content_length, expires_in_secs)
                    .ok()?,
            ),
        }
    }
}

pub const S3_DATA_PLANE_MESSAGE: &str =
    "the s3 object data plane needs the shared outbound http client lane";

pub enum Writer {
    Local(LocalWriter),
}

impl Writer {
    pub async fn write(&mut self, chunk: &[u8]) -> Result<(), StorageError> {
        match self {
            Self::Local(writer) => writer.write(chunk).await,
        }
    }

    pub async fn finish(self) -> Result<(), StorageError> {
        match self {
            Self::Local(writer) => writer.finish().await,
        }
    }

    pub async fn abort(self) {
        match self {
            Self::Local(writer) => writer.abort().await,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileStorage {
    driver: ValidatedDriver,
}

impl FileStorage {
    pub fn new(driver: ValidatedDriver) -> Self {
        Self { driver }
    }

    pub fn resource_path(folder: FileFolder, relative_path: &str) -> String {
        collapse_slashes(&format!("{}/{relative_path}", folder.as_str()))
    }

    pub fn storage_path(location: &FileLocation, resource_path: &str) -> String {
        collapse_slashes(&format!(
            "{}/{}/{}",
            location.workspace_id,
            location.application_universal_identifier,
            Self::resource_path(location.folder, resource_path)
        ))
    }

    pub fn validate(location: &FileLocation, resource_path: &str) -> Result<String, StorageError> {
        let validation = validate_file_path(resource_path, location.folder);

        if !validation.is_valid {
            return Err(StorageError::InvalidPath(
                validation.error.unwrap_or_else(|| String::from("Invalid resource path")),
            ));
        }

        let on_storage_path = Self::storage_path(location, resource_path);
        let prefix = format!(
            "{}/{}/{}/",
            location.workspace_id,
            location.application_universal_identifier,
            location.folder
        );

        if !on_storage_path.starts_with(&prefix) {
            return Err(StorageError::AccessDenied);
        }

        Ok(on_storage_path)
    }

    pub async fn start_write(
        &self,
        location: &FileLocation,
        resource_path: &str,
    ) -> Result<Writer, StorageError> {
        self.driver
            .start_write(&Self::validate(location, resource_path)?)
    }

    pub async fn metadata(
        &self,
        location: &FileLocation,
        resource_path: &str,
    ) -> Result<Option<StoredFile>, StorageError> {
        self.driver
            .metadata(&Self::validate(location, resource_path)?)
            .await
    }

    pub async fn read_prefix(
        &self,
        location: &FileLocation,
        resource_path: &str,
        byte_count: u64,
    ) -> Result<Vec<u8>, StorageError> {
        self.driver
            .read_prefix(&Self::validate(location, resource_path)?, byte_count)
            .await
    }

    pub async fn move_file(
        &self,
        from: &FileLocation,
        from_resource_path: &str,
        to: &FileLocation,
        to_resource_path: &str,
        if_match_checksum: Option<&str>,
    ) -> Result<(), StorageError> {
        self.driver
            .rename(
                &Self::validate(from, from_resource_path)?,
                &Self::validate(to, to_resource_path)?,
                if_match_checksum,
            )
            .await
    }

    pub async fn delete_file(
        &self,
        location: &FileLocation,
        resource_path: &str,
    ) -> Result<(), StorageError> {
        self.driver
            .delete(&Self::validate(location, resource_path)?)
            .await
    }

    pub fn presigned_upload_url(
        &self,
        location: &FileLocation,
        resource_path: &str,
        content_type: &str,
        content_length: u64,
        expires_in_secs: u64,
    ) -> Option<String> {
        self.driver
            .presigned_upload_url(
                &Self::validate(location, resource_path).ok()?,
                content_type,
                content_length,
                expires_in_secs,
            )
    }
}

impl ByteWindow {
    pub fn length(self) -> u64 {
        self.end_byte - self.start_byte + 1
    }
}

fn collapse_slashes(path: &str) -> String {
    let mut collapsed = String::with_capacity(path.len());
    let mut previous_slash = false;

    for character in path.chars() {
        if character == '/' {
            if previous_slash {
                continue;
            }

            previous_slash = true;
        } else {
            previous_slash = false;
        }

        collapsed.push(character);
    }

    collapsed
}
