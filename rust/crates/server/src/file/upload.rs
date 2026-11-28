use axum::body::Body;
use axum::extract::Path as AxumPath;
use axum::extract::Query;
use axum::extract::Request;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::put;
use http_body_util::BodyExt;
use chrono::DateTime;
use chrono::Utc;
use serde_json::json;
use serde_json::Value;
use uuid::Uuid;

use crate::auth::jwt_service::TokenType;
use crate::file::folder::FileFolder;
use crate::file::mime::extract_file_info;
use crate::file::storage::FileLocation;
use crate::file::storage::FileStorage;
use crate::file::storage::RawDriver;
use crate::file::storage::StorageError;
use crate::file::storage::ValidatedDriver;
use crate::file::validation::build_pending_upload_resource_path;
use crate::file::validation::extname;
use crate::file::validation::file_extension;
use crate::file::validation::remove_file_folder_from_file_entity_path;
use crate::file::validation::validate_file_path;
use crate::file::FileUrlSigner;
use crate::metadata::api::errors::GraphQlError;
use crate::metadata::api::graphql::FieldSelection;
use crate::state::AppState;

pub const MAX_SANITIZABLE_SVG_BYTES: u64 = 3 * 1024 * 1024;
pub const FILE_CONTENT_SNIFF_BYTE_COUNT: u64 = 64 * 1024;
pub const COMPLETE_FILE_UPLOAD_DEADLINE_SECS: u64 = 60;
pub const PENDING_FILE_MAX_AGE_SECS: i64 = 24 * 60 * 60;
pub const RECORD_EXPORT_FILE_MAX_AGE_SECS: i64 = 60 * 60;
pub const PENDING_FILE_CLEANUP_BATCH_SIZE: i64 = 200;
pub const DIRECT_UPLOAD_CONTENT_TYPE: &str = "application/octet-stream";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadErrorCode {
    BadRequest,
    FileNotFound,
    FileNotUploaded,
    FileSizeMismatch,
    FileTooLarge,
    StorageTimeout,
    StorageInconsistent,
}

impl UploadErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BadRequest => "BAD_REQUEST",
            Self::FileNotFound => "FILE_NOT_FOUND",
            Self::FileNotUploaded => "FILE_NOT_UPLOADED",
            Self::FileSizeMismatch => "FILE_SIZE_MISMATCH",
            Self::FileTooLarge => "FILE_TOO_LARGE",
            Self::StorageTimeout => "STORAGE_TIMEOUT",
            Self::StorageInconsistent => "STORAGE_INCONSISTENT",
        }
    }
}

#[derive(Debug, Clone)]
pub struct UploadError {
    pub code: UploadErrorCode,
    pub message: String,
    pub user_friendly: &'static str,
}

impl UploadError {
    fn new(code: UploadErrorCode, message: impl Into<String>, user_friendly: &'static str) -> Self {
        Self {
            code,
            message: message.into(),
            user_friendly,
        }
    }

    pub fn status(&self) -> StatusCode {
        match self.code {
            UploadErrorCode::FileNotFound => StatusCode::NOT_FOUND,
            UploadErrorCode::FileTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            UploadErrorCode::StorageTimeout => StatusCode::GATEWAY_TIMEOUT,
            UploadErrorCode::StorageInconsistent => StatusCode::BAD_GATEWAY,
            UploadErrorCode::BadRequest
            | UploadErrorCode::FileNotUploaded
            | UploadErrorCode::FileSizeMismatch => StatusCode::BAD_REQUEST,
        }
    }

    pub fn graphql(&self) -> GraphQlError {
        let mapped = match self.code {
            UploadErrorCode::FileNotFound => GraphQlError::not_found(self.message.clone()),
            UploadErrorCode::StorageInconsistent => GraphQlError::internal(self.message.clone()),
            UploadErrorCode::StorageTimeout => {
                GraphQlError::new(self.message.clone(), "GRAPHQL_TIMEOUT")
            }
            _ => GraphQlError::user_input(self.message.clone()),
        };

        mapped.with_extensions(json!({
            "subCode": self.code.as_str(),
            "userFriendlyMessage": { "message": self.user_friendly },
        }))
    }
}

impl From<StorageError> for UploadError {
    fn from(error: StorageError) -> Self {
        match error {
            StorageError::AccessDenied | StorageError::InvalidPath(_) => Self::new(
                UploadErrorCode::BadRequest,
                error.to_string(),
                "This file path is not allowed.",
            ),
            StorageError::FileNotFound => {
                Self::new(UploadErrorCode::FileNotFound, error.to_string(), "File not found.")
            }
            StorageError::PreconditionFailed(message) => Self::new(
                UploadErrorCode::StorageInconsistent,
                message,
                "File storage did not confirm the processed file. Please retry.",
            ),
            StorageError::Backend(message) => Self::new(
                UploadErrorCode::StorageInconsistent,
                message,
                "File storage did not respond. Please retry.",
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UploadTarget {
    pub file_id: Uuid,
    pub upload_url: String,
    pub content_type: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct StoredFileRow {
    id: Uuid,
    path: String,
    size: i64,
    mime_type: String,
    status: String,
    application_id: Uuid,
    settings: Option<Value>,
}

#[derive(Debug, Clone)]
struct UploadLocationParts {
    application_id: Uuid,
    application_universal_identifier: Uuid,
    folder: FileFolder,
    resource_path: String,
}

impl UploadLocationParts {
    fn location(&self, workspace_id: Uuid) -> FileLocation {
        FileLocation {
            workspace_id,
            application_universal_identifier: self.application_universal_identifier,
            folder: self.folder,
        }
    }
}

pub fn router() -> axum::Router<AppState> {
    axum::Router::new().route("/file-upload/{id}", put(upload_by_id))
}

pub fn file_storage(state: &AppState) -> FileStorage {
    FileStorage::new(ValidatedDriver::new(match storage_type().as_str() {
        "s3" => RawDriver::S3(s3_driver()),
        _ => RawDriver::Local(crate::file::drivers::local::LocalDriver::new(
            state.config.storage_local_path.clone(),
        )),
    }))
}

fn storage_type() -> String {
    std::env::var("STORAGE_TYPE")
        .unwrap_or_else(|_| String::from("local"))
        .trim()
        .to_ascii_lowercase()
}

fn presigned_url_expires_in() -> u64 {
    std::env::var("STORAGE_S3_PRESIGNED_URL_EXPIRES_IN")
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(900)
}

fn s3_driver() -> crate::file::drivers::s3::S3Driver {
    let credentials = match (
        std::env::var("STORAGE_S3_ACCESS_KEY_ID").ok(),
        std::env::var("STORAGE_S3_SECRET_ACCESS_KEY").ok(),
    ) {
        (Some(access_key_id), Some(secret_access_key)) => {
            Some(crate::file::drivers::s3::S3Credentials {
                access_key_id,
                secret_access_key,
            })
        }
        _ => None,
    };

    crate::file::drivers::s3::S3Driver {
        bucket: std::env::var("STORAGE_S3_NAME").unwrap_or_default(),
        region: std::env::var("STORAGE_S3_REGION").unwrap_or_default(),
        endpoint: env_non_empty("STORAGE_S3_ENDPOINT"),
        presign_base: env_non_empty("STORAGE_S3_PRESIGNED_URL_BASE"),
        credentials,
    }
}

fn env_non_empty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|value| !value.trim().is_empty())
}

pub async fn create_file_upload(
    state: &AppState,
    workspace_id: Uuid,
    filename: &str,
    size: i64,
    folder: FileFolder,
    field_metadata_id: Option<Uuid>,
    field_metadata_universal_identifier: Option<Uuid>,
) -> Result<UploadTarget, UploadError> {
    if !FileFolder::DIRECT_UPLOAD.contains(&folder) {
        return Err(UploadError::new(
            UploadErrorCode::BadRequest,
            format!("Direct upload is not supported for file folder {folder}"),
            "Direct upload is not supported for this file type.",
        ));
    }

    assert_valid_direct_upload_size(folder, size)?;

    let extension = file_extension(filename);

    if extension.eq_ignore_ascii_case("svg") && size as u64 > MAX_SANITIZABLE_SVG_BYTES {
        return Err(svg_too_large(format!(
            "declared size {size} exceeds the {MAX_SANITIZABLE_SVG_BYTES} byte limit"
        )));
    }

    let file_id = Uuid::new_v4();
    let name = if extension.is_empty() {
        file_id.to_string()
    } else {
        format!("{file_id}.{extension}")
    };
    let location = resolve_upload_location(
        state,
        workspace_id,
        folder,
        &name,
        field_metadata_id,
        field_metadata_universal_identifier,
    )
    .await?;

    let path = FileStorage::resource_path(folder, &location.resource_path);
    let stored_id = insert_pending_file(
        state,
        workspace_id,
        location.application_id,
        file_id,
        &path,
        size,
    )
    .await?;

    build_upload_target(
        state,
        workspace_id,
        stored_id,
        folder,
        location.application_universal_identifier,
        &location.resource_path,
        DIRECT_UPLOAD_CONTENT_TYPE.to_string(),
        size as u64,
    )
    .await
}

fn assert_valid_direct_upload_size(folder: FileFolder, size: i64) -> Result<(), UploadError> {
    let maximum = folder.max_file_size_bytes() as i64;

    if size <= 0 || size > maximum {
        return Err(UploadError::new(
            UploadErrorCode::FileTooLarge,
            format!("Invalid file size {size} (max {maximum} bytes)"),
            "The file is empty or exceeds the maximum allowed size.",
        ));
    }

    Ok(())
}

fn svg_too_large(detail: String) -> UploadError {
    UploadError::new(
        UploadErrorCode::FileTooLarge,
        format!("SVG cannot be sanitized: {detail}"),
        "This SVG is too large to be processed.",
    )
}

async fn resolve_upload_location(
    state: &AppState,
    workspace_id: Uuid,
    folder: FileFolder,
    name: &str,
    field_metadata_id: Option<Uuid>,
    field_metadata_universal_identifier: Option<Uuid>,
) -> Result<UploadLocationParts, UploadError> {
    if folder == FileFolder::FilesField {
        let identifier = match (field_metadata_id, field_metadata_universal_identifier) {
            (None, None) => {
                return Err(UploadError::new(
                    UploadErrorCode::BadRequest,
                    "fieldMetadataId or fieldMetadataUniversalIdentifier must be provided",
                    "fieldMetadataId or fieldMetadataUniversalIdentifier must be provided",
                ))
            }
            (Some(id), _) => ("id", id),
            (None, Some(universal_identifier)) => ("universalIdentifier", universal_identifier),
        };
        let row = sqlx::query_as::<_, (Uuid, Uuid)>(
            "SELECT \"universalIdentifier\", \"applicationId\" FROM core.\"fieldMetadata\" \
             WHERE id = $2 AND \"workspaceId\" = $1",
        )
        .bind(workspace_id)
        .bind(identifier.1)
        .fetch_optional(&state.core_pool)
        .await
        .map_err(|error| UploadError::new(
            UploadErrorCode::StorageInconsistent,
            error.to_string(),
            "File metadata could not be read.",
        ))?
        .ok_or_else(|| {
            UploadError::new(
                UploadErrorCode::FileNotFound,
                "Field metadata was not found",
                "Field metadata was not found.",
            )
        })?;
        let application = application_by_id(state, workspace_id, row.1).await?;

        return Ok(UploadLocationParts {
            application_id: row.1,
            application_universal_identifier: application,
            folder,
            resource_path: format!("{}/{}", row.0, name),
        });
    }

    let row = sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT \"workspaceCustomApplicationId\" FROM core.workspace WHERE id = $1",
    )
    .bind(workspace_id)
    .fetch_optional(&state.core_pool)
    .await
    .map_err(|error| UploadError::new(
        UploadErrorCode::StorageInconsistent,
        error.to_string(),
        "Workspace could not be read.",
    ))?
    .ok_or_else(|| UploadError::new(
        UploadErrorCode::FileNotFound,
        format!("Could not find workspace {workspace_id}"),
        "Workspace was not found.",
    ))?
    .ok_or_else(|| UploadError::new(
        UploadErrorCode::FileNotFound,
        format!("Could not find workspace custom application for {workspace_id}"),
        "Workspace application was not found.",
    ))?;

    Ok(UploadLocationParts {
        application_universal_identifier: application_by_id(state, workspace_id, row).await?,
        application_id: row,
        folder,
        resource_path: name.to_string(),
    })
}

async fn application_by_id(
    state: &AppState,
    workspace_id: Uuid,
    application_id: Uuid,
) -> Result<Uuid, UploadError> {
    sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT \"universalIdentifier\" FROM core.application \
         WHERE id = $1 AND \"workspaceId\" = $2 AND \"deletedAt\" IS NULL",
    )
    .bind(application_id)
    .bind(workspace_id)
    .fetch_optional(&state.core_pool)
    .await
    .map_err(|error| UploadError::new(
        UploadErrorCode::StorageInconsistent,
        error.to_string(),
        "Application could not be read.",
    ))?
    .ok_or_else(|| UploadError::new(
        UploadErrorCode::FileNotFound,
        format!("Application with id \"{application_id}\" not found"),
        "Application was not found.",
    ))?
    .ok_or_else(|| UploadError::new(
        UploadErrorCode::FileNotFound,
        format!("Application with id \"{application_id}\" not found"),
        "Application was not found.",
    ))
}

async fn insert_pending_file(
    state: &AppState,
    workspace_id: Uuid,
    application_id: Uuid,
    file_id: Uuid,
    path: &str,
    size: i64,
) -> Result<Uuid, UploadError> {
    let settings = json!({ "isTemporaryFile": true, "toDelete": false });
    let row = sqlx::query_as::<_, (Uuid,)>(
        "INSERT INTO core.file \
         (id, size, \"workspaceId\", \"applicationId\", path, \"mimeType\", status, settings, \"contentSha256\") \
         VALUES ($1, $2, $3, $4, $5, 'application/octet-stream', 'PENDING', $6, NULL) \
         ON CONFLICT (\"applicationId\", \"workspaceId\", path) DO UPDATE \
         SET size = EXCLUDED.size, \"mimeType\" = 'application/octet-stream', status = 'PENDING', \
             settings = EXCLUDED.settings, \"contentSha256\" = NULL, \"deletedAt\" = NULL \
         RETURNING id",
    )
    .bind(file_id)
    .bind(size)
    .bind(workspace_id)
    .bind(application_id)
    .bind(path)
    .bind(settings)
    .fetch_one(&state.core_pool)
    .await
    .map_err(|error| UploadError::new(
        UploadErrorCode::StorageInconsistent,
        error.to_string(),
        "The file could not be declared.",
    ))?;

    Ok(row.0)
}

pub async fn build_upload_target(
    state: &AppState,
    workspace_id: Uuid,
    file_id: Uuid,
    folder: FileFolder,
    application_universal_identifier: Uuid,
    resource_path: &str,
    content_type: String,
    size: u64,
) -> Result<UploadTarget, UploadError> {
    let location = FileLocation {
        workspace_id,
        application_universal_identifier,
        folder,
    };
    let pending_resource_path = build_pending_upload_resource_path(file_id, resource_path);
    let pending_validation = validate_file_path(&pending_resource_path, folder);

    if !pending_validation.is_valid {
        return Err(UploadError::new(
            UploadErrorCode::BadRequest,
            format!(
                "Resource path \"{resource_path}\" leaves no room for the pending upload prefix: {}",
                pending_validation.error.unwrap_or_default()
            ),
            "This file path is too long to upload.",
        ));
    }

    let expires_in_secs = presigned_url_expires_in();
    let expires_at = Utc::now() + chrono::Duration::seconds(expires_in_secs as i64);
    let storage = file_storage(state);

    if let Some(presigned) = storage.presigned_upload_url(
        &location,
        &pending_resource_path,
        &content_type,
        size,
        expires_in_secs,
    ) {
        return Ok(UploadTarget {
            file_id,
            upload_url: presigned,
            content_type,
            expires_at,
        });
    }

    let token = state
        .jwt
        .sign(
            json!({
                "workspaceId": workspace_id.to_string(),
                "fileId": file_id.to_string(),
                "sub": workspace_id.to_string(),
            }),
            TokenType::FileUpload,
            expires_in_secs,
            None,
        )
        .await
        .map_err(|error| UploadError::new(
            UploadErrorCode::StorageInconsistent,
            error.to_string(),
            "The upload target could not be signed.",
        ))?;

    Ok(UploadTarget {
        file_id,
        upload_url: format!(
            "{}/file-upload/{file_id}?token={token}",
            state.config.server_url.trim_end_matches('/')
        ),
        content_type: DIRECT_UPLOAD_CONTENT_TYPE.to_string(),
        expires_at,
    })
}

async fn upload_by_id(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
    Query(parameters): Query<std::collections::HashMap<String, String>>,
    request: Request,
) -> Response {
    let file_id = match Uuid::parse_str(&id) {
        Ok(file_id) => file_id,
        Err(_) => return upload_error(UploadError::new(
            UploadErrorCode::FileNotFound,
            format!("File not found: {id}"),
            "File not found.",
        )),
    };
    let Some(token) = parameters.get("token") else {
        return (StatusCode::FORBIDDEN, Body::empty()).into_response();
    };
    let payload = match state.jwt.verify_typed(token, TokenType::FileUpload).await {
        Ok(payload) => payload,
        Err(_) => return (StatusCode::FORBIDDEN, Body::empty()).into_response(),
    };

    if payload.get("fileId").and_then(Value::as_str) != Some(file_id.to_string().as_str()) {
        return (StatusCode::FORBIDDEN, Body::empty()).into_response();
    }

    let Some(workspace_id) = payload
        .get("workspaceId")
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
    else {
        return (StatusCode::FORBIDDEN, Body::empty()).into_response();
    };

    match receive_file_stream(&state, workspace_id, file_id, request.into_body()).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => upload_error(error),
    }
}

fn upload_error(error: UploadError) -> Response {
    (
        error.status(),
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json",
        )],
        json!({
            "statusCode": error.status().as_u16(),
            "code": error.code.as_str(),
            "message": error.message,
            "userFriendlyMessage": error.user_friendly,
        })
            .to_string(),
    )
        .into_response()
}

pub async fn receive_file_stream(
    state: &AppState,
    workspace_id: Uuid,
    file_id: Uuid,
    mut body: Body,
) -> Result<(), UploadError> {
    let file = find_file(state, workspace_id, file_id).await?;

    if file.status != "PENDING" {
        return Err(UploadError::new(
            UploadErrorCode::BadRequest,
            format!("File {file_id} is not awaiting an upload"),
            "This file has already been uploaded.",
        ));
    }

    let location = file_location(state, workspace_id, &file).await?;
    let declared_size = file.size;

    if declared_size <= 0 {
        return Err(UploadError::new(
            UploadErrorCode::FileSizeMismatch,
            format!("Invalid declared size {declared_size}"),
            "The uploaded file does not match the declared size. Please retry the upload.",
        ));
    }

    let pending_resource_path = build_pending_upload_resource_path(file_id, &location.resource_path);
    let mut writer = file_storage(state)
        .start_write(&location.location(workspace_id), &pending_resource_path)
        .await?;
    let mut received = 0_i64;

    while let Some(frame) = body.frame().await {
        let chunk = match frame {
            Ok(frame) => frame.into_data(),
            Err(_) => {
                writer.abort().await;

                return Err(UploadError::new(
                    UploadErrorCode::BadRequest,
                    "The upload body could not be read",
                    "The upload was interrupted. Please retry.",
                ));
            }
        };

        let Ok(chunk) = chunk else {
            continue;
        };

        received += chunk.len() as i64;

        if received > declared_size {
            writer.abort().await;

            return Err(UploadError::new(
                UploadErrorCode::FileTooLarge,
                format!("Upload exceeds declared size of {declared_size} bytes"),
                "The uploaded file is larger than declared.",
            ));
        }

        if let Err(error) = writer.write(&chunk).await {
            writer.abort().await;

            return Err(error.into());
        }
    }

    if received != declared_size {
        writer.abort().await;

        return Err(UploadError::new(
            UploadErrorCode::FileSizeMismatch,
            format!("Uploaded {received} bytes but {declared_size} were declared"),
            "The uploaded file does not match the declared size. Please retry the upload.",
        ));
    }

    writer.finish().await.map_err(UploadError::from)
}

async fn find_file(
    state: &AppState,
    workspace_id: Uuid,
    file_id: Uuid,
) -> Result<StoredFileRow, UploadError> {
    sqlx::query_as::<_, StoredFileRow>(
        "SELECT id, path, size, \"mimeType\", status::text AS status, \"applicationId\", settings \
         FROM core.file WHERE id = $1 AND \"workspaceId\" = $2",
    )
    .bind(file_id)
    .bind(workspace_id)
    .fetch_optional(&state.core_pool)
    .await
    .map_err(|error| UploadError::new(
        UploadErrorCode::StorageInconsistent,
        error.to_string(),
        "The file record could not be read.",
    ))?
    .ok_or_else(|| {
        UploadError::new(
            UploadErrorCode::FileNotFound,
            format!("File not found: {file_id}"),
            "File not found.",
        )
    })
}

async fn file_location(
    state: &AppState,
    workspace_id: Uuid,
    file: &StoredFileRow,
) -> Result<UploadLocationParts, UploadError> {
    let folder = FileFolder::from_path(&file.path).ok_or_else(|| {
        UploadError::new(
            UploadErrorCode::BadRequest,
            format!("File folder {} is not allowed", file.path),
            "This file type is not supported.",
        )
    })?;
    let application_universal_identifier =
        application_by_id(state, workspace_id, file.application_id).await?;

    Ok(UploadLocationParts {
        application_id: file.application_id,
        application_universal_identifier,
        folder,
        resource_path: remove_file_folder_from_file_entity_path(&file.path).map_err(|message| {
            UploadError::new(UploadErrorCode::BadRequest, message, "This file type is not supported.")
        })?,
    })
}

pub async fn complete_file_upload(
    state: &AppState,
    workspace_id: Uuid,
    file_id: Uuid,
    dedicated_folder: Option<FileFolder>,
) -> Result<CompletedUpload, UploadError> {
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(COMPLETE_FILE_UPLOAD_DEADLINE_SECS),
        complete_file_upload_inner(state, workspace_id, file_id, dedicated_folder),
    )
    .await;

    match outcome {
        Ok(result) => result,
        Err(_) => Err(UploadError::new(
            UploadErrorCode::StorageTimeout,
            format!(
                "Completion of file {file_id} exceeded {COMPLETE_FILE_UPLOAD_DEADLINE_SECS}s waiting on storage"
            ),
            "File storage took too long to respond. Please retry.",
        )),
    }
}

#[derive(Debug, Clone)]
pub struct CompletedUpload {
    pub id: Uuid,
    pub path: String,
    pub size: i64,
    pub created_at: DateTime<Utc>,
    pub mime_type: String,
    pub url: String,
}

async fn complete_file_upload_inner(
    state: &AppState,
    workspace_id: Uuid,
    file_id: Uuid,
    dedicated_folder: Option<FileFolder>,
) -> Result<CompletedUpload, UploadError> {
    let file = find_file(state, workspace_id, file_id).await?;
    let Some(folder) = FileFolder::from_path(&file.path) else {
        return Err(UploadError::new(
            UploadErrorCode::FileNotFound,
            format!("File not found: {file_id}"),
            "File not found.",
        ));
    };

    if !FileFolder::DIRECT_UPLOAD.contains(&folder) {
        return Err(UploadError::new(
            UploadErrorCode::FileNotFound,
            format!("File not found: {file_id}"),
            "File not found.",
        ));
    }

    if Some(folder) != dedicated_folder && FileFolder::DEDICATED_COMPLETION.contains(&folder) {
        return Err(UploadError::new(
            UploadErrorCode::BadRequest,
            format!(
                "File {file_id} in folder {folder} is completed by its dedicated mutation"
            ),
            "This file must be completed with its dedicated mutation.",
        ));
    }

    if file.status == "UPLOADED" {
        if !temporary_file(&file.settings) {
            return Err(UploadError::new(
                UploadErrorCode::BadRequest,
                format!("File {file_id} is not awaiting an upload confirmation"),
                "This file upload has already been finalized.",
            ));
        }

        return signed_completion(state, &file, folder, workspace_id, file.size, &file.mime_type)
            .await;
    }

    let location = file_location(state, workspace_id, &file).await?;
    let storage = file_storage(state);
    let pending_resource_path = build_pending_upload_resource_path(file_id, &location.resource_path);
    let pending_location = location.location(workspace_id);
    let metadata = storage
        .metadata(&pending_location, &pending_resource_path)
        .await?
        .ok_or_else(|| {
            UploadError::new(
                UploadErrorCode::FileNotUploaded,
                format!("File \"{}\" has not been uploaded to storage yet.", file.path),
                "The file has not been uploaded yet. Please upload it before confirming.",
            )
        })?;
    let declared_size = file.size;

    if metadata.size as i64 != declared_size {
        return Err(UploadError::new(
            UploadErrorCode::FileSizeMismatch,
            format!(
                "File \"{}\" has {} bytes in storage but {declared_size} were declared.",
                file.path, metadata.size
            ),
            "The uploaded file does not match the declared size. Please retry the upload.",
        ));
    }

    let prefix = storage
        .read_prefix(&pending_location, &pending_resource_path, FILE_CONTENT_SNIFF_BYTE_COUNT)
        .await?;
    let info = extract_file_info(&prefix, &file.path).map_err(|error| {
        UploadError::new(
            UploadErrorCode::BadRequest,
            error.message(),
            "The file extension doesn't match the file content. Please check that your file is not corrupted and has the correct extension.",
        )
    })?;

    if !folder.allows_mime_type(&info.mime_type) {
        return Err(UploadError::new(
            UploadErrorCode::BadRequest,
            format!(
                "MIME type {} is not allowed in file folder {folder}",
                info.mime_type
            ),
            "This file format is not supported.",
        ));
    }

    let mut size = metadata.size as i64;

    if info.mime_type == "image/svg+xml" {
        if size as u64 > MAX_SANITIZABLE_SVG_BYTES {
            return Err(svg_too_large(format!(
                "storage reports {size} bytes, above the {MAX_SANITIZABLE_SVG_BYTES} byte limit"
            )));
        }
    }

    storage
        .move_file(
            &pending_location,
            &pending_resource_path,
            &pending_location,
            &location.resource_path,
            metadata.checksum.as_deref(),
        )
        .await?;

    let affected = sqlx::query(
        "UPDATE core.file SET status = 'UPLOADED'::core.file_status_enum, \"mimeType\" = $3, size = $4 \
         WHERE id = $1 AND status = 'PENDING'::core.file_status_enum",
    )
    .bind(file_id)
    .bind(workspace_id)
    .bind(&info.mime_type)
    .bind(size)
    .execute(&state.core_pool)
    .await
    .map_err(|error| UploadError::new(
        UploadErrorCode::StorageInconsistent,
        error.to_string(),
        "The file could not be confirmed.",
    ))?
    .rows_affected();

    if affected == 0 {
        return Err(UploadError::new(
            UploadErrorCode::FileNotFound,
            format!("File {file_id} was reaped while its upload was being completed"),
            "This upload expired before it was confirmed. Please upload the file again.",
        ));
    }

    size = declared_size.max(size);

    signed_completion(state, &file, folder, workspace_id, size, &info.mime_type).await
}

fn temporary_file(settings: &Option<Value>) -> bool {
    settings
        .as_ref()
        .and_then(|value| value.get("isTemporaryFile"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

async fn signed_completion(
    state: &AppState,
    file: &StoredFileRow,
    folder: FileFolder,
    workspace_id: Uuid,
    size: i64,
    mime_type: &str,
) -> Result<CompletedUpload, UploadError> {
    let created_at = sqlx::query_as::<_, (DateTime<Utc>,)>("SELECT \"createdAt\" FROM core.file WHERE id = $1")
        .bind(file.id)
        .fetch_one(&state.core_pool)
        .await
        .map_err(|error| UploadError::new(
            UploadErrorCode::StorageInconsistent,
            error.to_string(),
            "The file record could not be read.",
        ))?
        .0;
    let signer = FileUrlSigner::new(
        state.jwt.clone(),
        state.config.server_url.clone(),
        state.config.file_token_expires_in_secs,
    );
    let url = signer
        .signed_url_in_folder(file.id, workspace_id, folder)
        .await
        .map_err(|error| UploadError::new(
            UploadErrorCode::StorageInconsistent,
            error.to_string(),
            "The file url could not be signed.",
        ))?;

    Ok(CompletedUpload {
        id: file.id,
        path: file.path.clone(),
        size,
        created_at,
        mime_type: mime_type.to_string(),
        url,
    })
}

pub fn project_completed(upload: &CompletedUpload, selection: &[FieldSelection]) -> Value {
    let record = json!({
        "id": upload.id.to_string(),
        "path": upload.path,
        "size": upload.size,
        "createdAt": upload.created_at.to_rfc3339(),
        "url": upload.url,
    });

    project(&record, selection, "FileWithSignedUrl")
}

pub fn project_target(target: &UploadTarget, selection: &[FieldSelection]) -> Value {
    let record = json!({
        "fileId": target.file_id.to_string(),
        "uploadUrl": target.upload_url,
        "contentType": target.content_type,
        "expiresAt": target.expires_at.to_rfc3339(),
    });

    project(&record, selection, "FileUploadTarget")
}

fn project(record: &Value, selection: &[FieldSelection], typename: &str) -> Value {
    let mut projected = serde_json::Map::new();

    projected.insert("__typename".to_string(), json!(typename));

    for field in selection {
        if let Some(value) = record.get(&field.name) {
            projected.insert(field.response_key().to_string(), value.clone());
        }
    }

    Value::Object(projected)
}

pub async fn graphql_create_file_upload(
    state: &AppState,
    workspace_id: Uuid,
    field: &FieldSelection,
) -> Result<Value, GraphQlError> {
    let filename = argument_string(field, "filename")?;
    let size = argument_number(field, "size")?;
    let folder = argument_folder(field)?;
    let field_metadata_id = argument_optional_uuid(field, "fieldMetadataId")?;
    let field_metadata_universal_identifier =
        argument_optional_uuid(field, "fieldMetadataUniversalIdentifier")?;

    if extname(&filename).is_empty() && !filename.is_empty() && !folder_allows_extensionless(&filename) {
        return Err(GraphQlError::user_input(format!(
            "Filename '{filename}' is not allowed"
        ))
        .with_sub_code("BAD_REQUEST"));
    }

    let target = create_file_upload(
        state,
        workspace_id,
        &filename,
        size,
        folder,
        field_metadata_id,
        field_metadata_universal_identifier,
    )
    .await
    .map_err(|error| error.graphql())?;

    Ok(project_target(&target, &field.selection))
}

fn folder_allows_extensionless(filename: &str) -> bool {
    !filename.contains('/') && !filename.contains('\\') && !filename.contains('\0')
}

pub async fn graphql_complete_file_upload(
    state: &AppState,
    workspace_id: Uuid,
    field: &FieldSelection,
) -> Result<Value, GraphQlError> {
    let file_id = argument_uuid(field, "fileId")?;
    let upload = complete_file_upload(state, workspace_id, file_id, None)
        .await
        .map_err(|error| error.graphql())?;

    Ok(project_completed(&upload, &field.selection))
}

fn argument_string(field: &FieldSelection, name: &str) -> Result<String, GraphQlError> {
    field
        .argument(name)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| {
            GraphQlError::user_input(format!("Argument \"{name}\" is required")).with_sub_code("BAD_USER_INPUT")
        })
}

fn argument_number(field: &FieldSelection, name: &str) -> Result<i64, GraphQlError> {
    let value = field.argument(name).ok_or_else(|| {
        GraphQlError::user_input(format!("Argument \"{name}\" is required")).with_sub_code("BAD_USER_INPUT")
    })?;

    value
        .as_i64()
        .or_else(|| value.as_f64().map(|number| number as i64))
        .ok_or_else(|| {
            GraphQlError::user_input(format!("Argument \"{name}\" must be a number"))
                .with_sub_code("BAD_USER_INPUT")
        })
}

fn argument_uuid(field: &FieldSelection, name: &str) -> Result<Uuid, GraphQlError> {
    argument_optional_uuid(field, name)?.ok_or_else(|| {
        GraphQlError::user_input(format!("Argument \"{name}\" is required")).with_sub_code("BAD_USER_INPUT")
    })
}

fn argument_optional_uuid(
    field: &FieldSelection,
    name: &str,
) -> Result<Option<Uuid>, GraphQlError> {
    match field.argument(name) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_str()
            .and_then(|raw| Uuid::parse_str(raw).ok())
            .map(Some)
            .ok_or_else(|| {
                GraphQlError::user_input(format!("Argument \"{name}\" must be a UUID"))
                    .with_sub_code("BAD_USER_INPUT")
            }),
    }
}

fn argument_folder(field: &FieldSelection) -> Result<FileFolder, GraphQlError> {
    let raw = argument_string(field, "fileFolder")?;

    FileFolder::parse(&raw).ok_or_else(|| {
        GraphQlError::user_input(format!(
            "Argument \"fileFolder\" got invalid value \"{raw}\""
        ))
        .with_sub_code("BAD_USER_INPUT")
    })
}

pub async fn cleanup_stale_files(state: &AppState) -> Result<i64, UploadError> {
    let storage = file_storage(state);
    let stale = sqlx::query_as::<_, StoredFileRow>(
        "SELECT id, path, size, \"mimeType\", status::text AS status, \"applicationId\", settings \
         FROM core.file \
         WHERE (status = 'PENDING'::core.file_status_enum \
                AND \"createdAt\" < now() - make_interval(secs => $1) \
                AND \"workspaceId\" IS NOT NULL \
                AND path NOT LIKE 'workspace-data-export/%') \
            OR (path LIKE 'record-export/%' \
                AND \"createdAt\" < now() - make_interval(secs => $2) \
                AND \"workspaceId\" IS NOT NULL) \
            OR (status = 'UPLOADED'::core.file_status_enum \
                AND \"workspaceId\" IS NOT NULL \
                AND settings @> '{\"isTemporaryFile\": true}' \
                AND \"settings\"->>'expiresAt' <= to_char(now(), 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"') \
                AND \"deletedAt\" IS NULL) \
         LIMIT $3",
    )
    .bind(PENDING_FILE_MAX_AGE_SECS)
    .bind(RECORD_EXPORT_FILE_MAX_AGE_SECS)
    .bind(PENDING_FILE_CLEANUP_BATCH_SIZE)
    .fetch_all(&state.core_pool)
    .await
    .map_err(|error| UploadError::new(
        UploadErrorCode::StorageInconsistent,
        error.to_string(),
        "Stale files could not be listed.",
    ))?;
    let mut deleted = 0_i64;

    for file in stale {
        let Some(workspace_id) = sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT \"workspaceId\" FROM core.file WHERE id = $1",
        )
        .bind(file.id)
        .fetch_optional(&state.core_pool)
        .await
        .ok()
        .flatten()
        .flatten() else {
            continue;
        };
        let Some(folder) = FileFolder::from_path(&file.path) else {
            continue;
        };

        if folder == FileFolder::RecordExport || (file.status == "UPLOADED" && temporary_file(&file.settings))
        {
            let application = application_by_id(state, workspace_id, file.application_id).await;
            let Ok(application_universal_identifier) = application else {
                continue;
            };
            let location = FileLocation {
                workspace_id,
                application_universal_identifier,
                folder,
            };

            if let Ok(resource_path) = remove_file_folder_from_file_entity_path(&file.path) {
                let _ = storage.delete_file(&location, &resource_path).await;
            }

            let affected = sqlx::query("DELETE FROM core.file WHERE id = $1")
                .bind(file.id)
                .execute(&state.core_pool)
                .await
                .map(|result| result.rows_affected())
                .unwrap_or_default();

            deleted += affected as i64;

            continue;
        }

        let affected = sqlx::query("DELETE FROM core.file WHERE id = $1 AND status = 'PENDING'::core.file_status_enum")
            .bind(file.id)
            .execute(&state.core_pool)
            .await
            .map(|result| result.rows_affected())
            .unwrap_or_default();

        if affected == 0 {
            continue;
        }

        deleted += affected as i64;

        let Ok(application_universal_identifier) =
            application_by_id(state, workspace_id, file.application_id).await
        else {
            continue;
        };
        let location = FileLocation {
            workspace_id,
            application_universal_identifier,
            folder,
        };
        let Ok(resource_path) = remove_file_folder_from_file_entity_path(&file.path) else {
            continue;
        };

        let _ = storage
            .delete_file(
                &location,
                &build_pending_upload_resource_path(file.id, &resource_path),
            )
            .await;
        let _ = storage.delete_file(&location, &resource_path).await;
    }

    Ok(deleted)
}

pub async fn cleanup_loop(state: AppState) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(3600)).await;

        match cleanup_stale_files(&state).await {
            Ok(deleted) if deleted > 0 => {
                tracing::info!(deleted, "stale file cleanup removed rows");
            }
            Ok(_) => {}
            Err(error) => tracing::warn!(code = error.code.as_str(), "stale file cleanup failed"),
        }
    }
}
