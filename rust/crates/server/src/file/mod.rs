pub mod drivers;
pub mod folder;
pub mod mime;
pub mod storage;
pub mod upload;
pub mod validation;

use std::collections::HashMap;
#[cfg(target_os = "linux")]
use std::os::fd::AsRawFd;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::Path as AxumPath;
use axum::extract::Query;
use axum::extract::State;
use axum::http::header;
use axum::http::HeaderMap;
use axum::http::HeaderValue;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use serde_json::json;
use serde_json::Value;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncSeekExt;
use tokio::io::SeekFrom;
use tokio_util::io::ReaderStream;
use uuid::Uuid;

use crate::auth::error::AuthError;
use crate::auth::error::AuthResult;
use crate::auth::jwt_service::JwtService;
use crate::auth::jwt_service::TokenType;
use crate::file::folder::FileFolder;
use crate::state::AppState;

#[derive(Clone)]
pub struct FileUrlSigner {
    jwt: Arc<JwtService>,
    server_url: String,
    expires_in_secs: u64,
}

impl FileUrlSigner {
    pub fn new(jwt: Arc<JwtService>, server_url: String, expires_in_secs: u64) -> Self {
        Self {
            jwt,
            server_url,
            expires_in_secs,
        }
    }

    pub(crate) async fn signed_url(&self, file_id: Uuid, workspace_id: Uuid) -> AuthResult<String> {
        self.signed_url_in_folder(file_id, workspace_id, FileFolder::CorePicture)
            .await
    }

    pub async fn signed_url_in_folder(
        &self,
        file_id: Uuid,
        workspace_id: Uuid,
        folder: FileFolder,
    ) -> AuthResult<String> {
        let token = self
            .jwt
            .sign(
                json!({
                    "workspaceId": workspace_id.to_string(),
                    "fileId": file_id.to_string(),
                    "sub": workspace_id.to_string(),
                }),
                TokenType::File,
                self.expires_in_secs,
                None,
            )
            .await?;
        Ok(format!(
            "{}/file/{folder}/{file_id}?token={token}",
            self.server_url.trim_end_matches('/')
        ))
    }
}

pub async fn sign_search_avatars(
    result: &mut Value,
    signer: &FileUrlSigner,
    workspace_id: Uuid,
) -> AuthResult<()> {
    let Some(edges) = result.get_mut("edges").and_then(Value::as_array_mut) else {
        return Ok(());
    };
    let mut signed_indexes = vec![false; edges.len()];
    let mut pending = Vec::new();
    for (index, edge) in edges.iter().enumerate() {
        let Some(node) = edge.get("node").and_then(Value::as_object) else {
            continue;
        };
        if node.get("objectNameSingular").and_then(Value::as_str) != Some("workspaceMember") {
            continue;
        }
        let raw_url = node
            .get("imageUrl")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if let Some(file_id) = core_picture_file_id(raw_url) {
            pending.push((index, file_id));
        }
    }
    let mut tasks = tokio::task::JoinSet::new();
    for (index, file_id) in pending {
        let signer = signer.clone();
        tasks.spawn(async move { (index, signer.signed_url(file_id, workspace_id).await) });
    }
    while let Some(task) = tasks.join_next().await {
        let (index, result) = task.map_err(|error| AuthError::internal(error.to_string()))?;
        let signed_url = result?;
        if let Some(node) = edges
            .get_mut(index)
            .and_then(|edge| edge.get_mut("node"))
            .and_then(Value::as_object_mut)
        {
            signed_indexes[index] = true;
            node.insert("imageUrl".to_string(), Value::String(signed_url));
        }
    }
    for (index, edge) in edges.iter_mut().enumerate() {
        let Some(node) = edge.get_mut("node").and_then(Value::as_object_mut) else {
            continue;
        };
        if !signed_indexes[index]
            && node.get("objectNameSingular").and_then(Value::as_str) == Some("workspaceMember")
        {
            node.insert("imageUrl".to_string(), Value::String(String::new()));
        }
    }
    Ok(())
}

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/file/{folder}/{id}", get(file_download))
        .merge(upload::router())
}

async fn file_download(
    State(state): State<AppState>,
    AxumPath((folder, id)): AxumPath<(String, String)>,
    Query(parameters): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    let (Some(folder), Ok(file_id)) = (FileFolder::parse(&folder), Uuid::parse_str(&id)) else {
        return file_error(StatusCode::NOT_FOUND);
    };
    let Some(token) = parameters.get("token") else {
        return file_error(StatusCode::FORBIDDEN);
    };
    let payload = match state
        .jwt
        .verify_typed_with_expiration(token, TokenType::File, folder.enforces_expiration())
        .await
    {
        Ok(payload) => payload,
        Err(_) => return file_error(StatusCode::FORBIDDEN),
    };
    let Some(workspace_id) = payload
        .get("workspaceId")
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
    else {
        return file_error(StatusCode::FORBIDDEN);
    };
    if payload.get("exp").and_then(Value::as_i64).is_none()
        || payload.get("fileId").and_then(Value::as_str) != Some(file_id.to_string().as_str())
        || payload.get("sub").and_then(Value::as_str) != Some(workspace_id.to_string().as_str())
    {
        return file_error(StatusCode::FORBIDDEN);
    }
    let record = match sqlx::query_as::<_, (i64, String, String, Uuid)>(
        "SELECT f.size, f.path, f.\"mimeType\", a.\"universalIdentifier\" \
         FROM core.file f JOIN core.application a ON a.id = f.\"applicationId\" \
         AND a.\"workspaceId\" = f.\"workspaceId\" \
         JOIN core.workspace w ON w.id = f.\"workspaceId\" \
         WHERE f.id = $1 AND f.\"workspaceId\" = $2 AND f.\"deletedAt\" IS NULL \
         AND a.\"deletedAt\" IS NULL AND f.status = 'UPLOADED' \
         AND f.\"applicationRegistrationId\" IS NULL AND f.\"isStaticAsset\" = false \
         AND (f.path LIKE 'core-picture/%' OR w.\"suspendedAt\" IS NULL) \
         AND f.path LIKE $3",
    )
    .bind(file_id)
    .bind(workspace_id)
    .bind(folder.pattern())
    .fetch_optional(&state.core_pool)
    .await
    {
        Ok(Some(record)) => record,
        Ok(None) => return file_error(StatusCode::NOT_FOUND),
        Err(_) => return file_error(StatusCode::INTERNAL_SERVER_ERROR),
    };
    if !folder.allows_mime_type(&record.2) {
        return file_error(StatusCode::NOT_FOUND);
    }
    let Ok(size) = u64::try_from(record.0) else {
        return file_error(StatusCode::NOT_FOUND);
    };
    let Some(relative_path) = safe_relative_path(&record.1) else {
        return file_error(StatusCode::NOT_FOUND);
    };
    let Some(path) = resolve_local_path(
        &state.config.storage_local_path,
        workspace_id,
        record.3,
        &relative_path,
    ) else {
        return file_error(StatusCode::NOT_FOUND);
    };
    let mut file = match File::open(&path).await {
        Ok(file) => file,
        Err(_) => return file_error(StatusCode::NOT_FOUND),
    };
    if !opened_file_within_storage(
        &file,
        &state.config.storage_local_path,
        workspace_id,
        record.3,
    ) {
        return file_error(StatusCode::NOT_FOUND);
    }
    let actual_size = match file.metadata().await {
        Ok(metadata) if metadata.is_file() => metadata.len(),
        _ => return file_error(StatusCode::NOT_FOUND),
    };
    if actual_size != size {
        return file_error(StatusCode::NOT_FOUND);
    }
    let range = parse_byte_range(
        headers
            .get(header::RANGE)
            .and_then(|value| value.to_str().ok()),
        size,
    );
    let (status, start, length) = match range {
        ByteRange::Full => (StatusCode::OK, 0, size),
        ByteRange::Partial { start, length } => (StatusCode::PARTIAL_CONTENT, start, length),
        ByteRange::Unsatisfiable => {
            let mut response = file_error(StatusCode::RANGE_NOT_SATISFIABLE);
            if let Ok(value) = HeaderValue::from_str(&format!("bytes */{size}")) {
                response.headers_mut().insert(header::CONTENT_RANGE, value);
            }
            return response;
        }
    };
    if start != 0 && file.seek(SeekFrom::Start(start)).await.is_err() {
        return file_error(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let mut response = Response::new(Body::from_stream(ReaderStream::new(file.take(length))));
    *response.status_mut() = status;
    let response_headers = response.headers_mut();
    response_headers.insert(
        header::CONTENT_TYPE,
        mime_header(&record.2)
            .unwrap_or_else(|| HeaderValue::from_static("application/octet-stream")),
    );
    response_headers.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&length.to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    );
    response_headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response_headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    if let Some(value) = folder
        .cache_control()
        .and_then(|value| HeaderValue::from_str(value).ok())
    {
        response_headers.insert(header::CACHE_CONTROL, value);
    }
    response_headers.insert(header::CONTENT_DISPOSITION, content_disposition(&record.2));
    if status == StatusCode::PARTIAL_CONTENT {
        if let Ok(value) =
            HeaderValue::from_str(&format!("bytes {}-{}/{}", start, start + length - 1, size))
        {
            response_headers.insert(header::CONTENT_RANGE, value);
        }
    }
    response
}

fn file_error(status: StatusCode) -> Response {
    (status, Body::empty()).into_response()
}

fn core_picture_file_id(raw_url: &str) -> Option<Uuid> {
    let url = reqwest::Url::parse(raw_url).ok()?;
    let mut segments = url.path_segments()?;
    if segments.next()? != "file" || segments.next()? != "core-picture" {
        return None;
    }
    let file_id = Uuid::parse_str(segments.next()?).ok()?;
    if segments.next().is_some() {
        return None;
    }
    Some(file_id)
}

fn safe_relative_path(raw_path: &str) -> Option<PathBuf> {
    if raw_path.is_empty() || raw_path.contains(['\0', '\\']) {
        return None;
    }
    let path = Path::new(raw_path);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir
                    | Component::RootDir
                    | Component::CurDir
                    | Component::Prefix(_)
            )
        })
    {
        return None;
    }
    Some(path.to_path_buf())
}

fn resolve_local_path(
    storage_root: &Path,
    workspace_id: Uuid,
    application_id: Uuid,
    relative_path: &Path,
) -> Option<PathBuf> {
    let root = storage_root.canonicalize().ok()?;
    let expected = root
        .join(workspace_id.to_string())
        .join(application_id.to_string())
        .canonicalize()
        .ok()?;
    let candidate = expected.join(relative_path).canonicalize().ok()?;
    (candidate.starts_with(&root) && candidate.starts_with(&expected)).then_some(candidate)
}

fn opened_file_within_storage(
    file: &File,
    storage_root: &Path,
    workspace_id: Uuid,
    application_id: Uuid,
) -> bool {
    let root = storage_root.canonicalize().ok();
    let expected = root.as_ref().and_then(|root| {
        root.join(workspace_id.to_string())
            .join(application_id.to_string())
            .canonicalize()
            .ok()
    });
    let (Some(root), Some(expected)) = (root, expected) else {
        return false;
    };
    if !expected.starts_with(&root) {
        return false;
    }
    #[cfg(target_os = "linux")]
    {
        std::fs::canonicalize(format!("/proc/self/fd/{}", file.as_raw_fd()))
            .map(|path| path.starts_with(&root) && path.starts_with(&expected))
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = file;
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ByteRange {
    Full,
    Partial { start: u64, length: u64 },
    Unsatisfiable,
}

fn parse_byte_range(value: Option<&str>, size: u64) -> ByteRange {
    let Some(value) = value else {
        return ByteRange::Full;
    };
    let Some(value) = value.strip_prefix("bytes=") else {
        return ByteRange::Full;
    };
    if value.contains(',') {
        return ByteRange::Full;
    }
    let Some((start, end)) = value.split_once('-') else {
        return ByteRange::Full;
    };
    if start.is_empty() {
        let Ok(suffix) = end.parse::<u64>() else {
            return ByteRange::Full;
        };
        if suffix == 0 || size == 0 {
            return ByteRange::Unsatisfiable;
        }
        let length = suffix.min(size);
        return ByteRange::Partial {
            start: size - length,
            length,
        };
    }
    let Ok(start) = start.parse::<u64>() else {
        return ByteRange::Full;
    };
    if start >= size {
        return ByteRange::Unsatisfiable;
    }
    let end = if end.is_empty() {
        size - 1
    } else {
        let Ok(end) = end.parse::<u64>() else {
            return ByteRange::Full;
        };
        if end < start {
            return ByteRange::Full;
        }
        end.min(size - 1)
    };
    ByteRange::Partial {
        start,
        length: end - start + 1,
    }
}

fn mime_header(value: &str) -> Option<HeaderValue> {
    let value = value.trim();
    if value.is_empty() || value.chars().any(|character| character.is_control()) {
        return None;
    }
    HeaderValue::from_str(value).ok()
}

fn content_disposition(mime_type: &str) -> HeaderValue {
    let inline = matches!(
        mime_type,
        "image/jpeg"
            | "image/png"
            | "image/gif"
            | "image/webp"
            | "image/avif"
            | "image/bmp"
            | "image/tiff"
            | "application/pdf"
            | "text/plain"
            | "audio/mpeg"
            | "audio/wav"
            | "audio/ogg"
            | "video/mp4"
            | "video/webm"
            | "video/ogg"
            | "image/x-icon"
    );
    if inline {
        HeaderValue::from_static("inline")
    } else {
        HeaderValue::from_static("attachment")
    }
}

#[cfg(test)]
mod db_tests {
    use super::*;
    use crate::query::runner::test_support::drop_workspace;
    use crate::query::runner::test_support::seeded_workspace;
    use crate::query::runner::test_support::DB_TEST_LOCK;

    #[tokio::test]
    async fn signed_local_files_enforce_workspace_folder_expiry_and_ranges() {
        let Ok(url) = std::env::var("NUDGE_TEST_DATABASE_URL") else {
            return;
        };
        let _guard = DB_TEST_LOCK.lock().await;
        let mut seeded = seeded_workspace(&url).await.expect("seeded file workspace");
        let root = std::env::temp_dir().join(format!("nudge-file-db-{}", Uuid::new_v4()));
        let mut config = (*seeded.state.config).clone();
        config.storage_local_path = root.clone();
        seeded.state.config = Arc::new(config);

        let application_id: (Uuid,) = sqlx::query_as(
            "SELECT \"workspaceCustomApplicationId\" FROM core.workspace WHERE id = $1",
        )
        .bind(seeded.workspace_id)
        .fetch_one(&seeded.state.core_pool)
        .await
        .expect("workspace application");
        let universal_id: (Uuid,) =
            sqlx::query_as("SELECT \"universalIdentifier\" FROM core.application WHERE id = $1")
                .bind(application_id.0)
                .fetch_one(&seeded.state.core_pool)
                .await
                .expect("application universal identifier");
        let application_directory = root
            .join(seeded.workspace_id.to_string())
            .join(universal_id.0.to_string());
        std::fs::create_dir_all(application_directory.join("core-picture"))
            .expect("creating picture directory");
        std::fs::create_dir_all(application_directory.join("files-field"))
            .expect("creating files directory");
        std::fs::write(
            application_directory.join("core-picture/avatar.png"),
            b"avatar-bytes",
        )
        .expect("writing avatar");
        std::fs::write(
            application_directory.join("files-field/note.txt"),
            b"note-bytes",
        )
        .expect("writing note");

        let picture_id = Uuid::new_v4();
        let files_id = Uuid::new_v4();
        insert_file(
            &seeded.state,
            seeded.workspace_id,
            application_id.0,
            picture_id,
            "core-picture/avatar.png",
            "image/png",
            b"avatar-bytes".len() as i64,
        )
        .await;
        insert_file(
            &seeded.state,
            seeded.workspace_id,
            application_id.0,
            files_id,
            "files-field/note.txt",
            "text/plain",
            b"note-bytes".len() as i64,
        )
        .await;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("binding file server");
        let address = listener.local_addr().expect("file server address");
        let server_state = seeded.state.clone();
        let server = tokio::spawn(async move {
            axum::serve(listener, crate::routes::router(server_state))
                .await
                .expect("file server");
        });
        let client = reqwest::Client::new();
        let token = file_token(&seeded.state, seeded.workspace_id, picture_id, 60).await;
        let response = client
            .get(format!(
                "http://{address}/file/core-picture/{picture_id}?token={token}"
            ))
            .send()
            .await
            .map_err(|_| "file request failed")
            .expect("picture response");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-type"], "image/png");
        assert_eq!(response.headers()["content-disposition"], "inline");
        assert_eq!(response.headers()["content-length"], "12");
        assert_eq!(response.headers()["x-content-type-options"], "nosniff");
        assert_eq!(response.headers()["accept-ranges"], "bytes");
        assert_eq!(
            response.bytes().await.expect("picture body").as_ref(),
            b"avatar-bytes".as_slice()
        );

        let response = client
            .get(format!(
                "http://{address}/file/core-picture/{picture_id}?token={token}"
            ))
            .header(header::RANGE, "bytes=2-5")
            .send()
            .await
            .map_err(|_| "file request failed")
            .expect("range response");
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(response.headers()["content-range"], "bytes 2-5/12");
        assert_eq!(
            response.bytes().await.expect("range body").as_ref(),
            b"atar".as_slice()
        );

        let expired_picture = file_token(&seeded.state, seeded.workspace_id, picture_id, 0).await;
        let response = client
            .get(format!(
                "http://{address}/file/core-picture/{picture_id}?token={expired_picture}"
            ))
            .send()
            .await
            .map_err(|_| "file request failed")
            .expect("expired picture response");
        assert_eq!(response.status(), StatusCode::OK);

        let expired_files = file_token(&seeded.state, seeded.workspace_id, files_id, 0).await;
        let response = client
            .get(format!(
                "http://{address}/file/files-field/{files_id}?token={expired_files}"
            ))
            .send()
            .await
            .map_err(|_| "file request failed")
            .expect("expired files response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let wrong_workspace = file_token(&seeded.state, Uuid::new_v4(), picture_id, 60).await;
        let response = client
            .get(format!(
                "http://{address}/file/core-picture/{picture_id}?token={wrong_workspace}"
            ))
            .send()
            .await
            .map_err(|_| "file request failed")
            .expect("cross-workspace response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let valid_files = file_token(&seeded.state, seeded.workspace_id, files_id, 60).await;
        let response = client
            .get(format!(
                "http://{address}/file/files-field/{files_id}?token={valid_files}"
            ))
            .header(header::ACCEPT_ENCODING, "gzip")
            .send()
            .await
            .map_err(|_| "file request failed")
            .expect("files response");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-disposition"], "inline");
        assert!(!response.headers().contains_key(header::CONTENT_ENCODING));
        assert_eq!(
            response.bytes().await.expect("files body").as_ref(),
            b"note-bytes".as_slice()
        );

        sqlx::query("UPDATE core.workspace SET \"suspendedAt\" = now() WHERE id = $1")
            .bind(seeded.workspace_id)
            .execute(&seeded.state.core_pool)
            .await
            .expect("suspending workspace");
        let response = client
            .get(format!(
                "http://{address}/file/files-field/{files_id}?token={valid_files}"
            ))
            .send()
            .await
            .map_err(|_| "file request failed")
            .expect("suspended files response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let response = client
            .get(format!(
                "http://{address}/file/core-picture/{picture_id}?token={token}"
            ))
            .send()
            .await
            .map_err(|_| "file request failed")
            .expect("suspended picture response");
        assert_eq!(response.status(), StatusCode::OK);
        sqlx::query("UPDATE core.workspace SET \"suspendedAt\" = NULL WHERE id = $1")
            .bind(seeded.workspace_id)
            .execute(&seeded.state.core_pool)
            .await
            .expect("restoring workspace");

        sqlx::query("UPDATE core.file SET \"deletedAt\" = now() WHERE id = $1")
            .bind(picture_id)
            .execute(&seeded.state.core_pool)
            .await
            .expect("deleting picture");
        let response = client
            .get(format!(
                "http://{address}/file/core-picture/{picture_id}?token={token}"
            ))
            .send()
            .await
            .map_err(|_| "file request failed")
            .expect("deleted picture response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let traversal_id = Uuid::new_v4();
        insert_file(
            &seeded.state,
            seeded.workspace_id,
            application_id.0,
            traversal_id,
            "core-picture/../../etc/passwd",
            "image/png",
            1,
        )
        .await;
        let traversal_token =
            file_token(&seeded.state, seeded.workspace_id, traversal_id, 60).await;
        let response = client
            .get(format!(
                "http://{address}/file/core-picture/{traversal_id}?token={traversal_token}"
            ))
            .send()
            .await
            .map_err(|_| "file request failed")
            .expect("traversal response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        server.abort();
        drop_workspace(&seeded).await;
        std::fs::remove_dir_all(root).expect("removing file fixture");
    }

    async fn insert_file(
        state: &AppState,
        workspace_id: Uuid,
        application_id: Uuid,
        id: Uuid,
        path: &str,
        mime_type: &str,
        size: i64,
    ) {
        sqlx::query(
            "INSERT INTO core.file \
             (id, size, \"workspaceId\", \"applicationId\", path, \"mimeType\") \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(id)
        .bind(size)
        .bind(workspace_id)
        .bind(application_id)
        .bind(path)
        .bind(mime_type)
        .execute(&state.core_pool)
        .await
        .expect("inserting file fixture");
    }

    async fn file_token(
        state: &AppState,
        workspace_id: Uuid,
        file_id: Uuid,
        expires_in_secs: u64,
    ) -> String {
        state
            .jwt
            .sign(
                json!({
                    "workspaceId": workspace_id.to_string(),
                    "fileId": file_id.to_string(),
                    "sub": workspace_id.to_string(),
                }),
                TokenType::File,
                expires_in_secs,
                None,
            )
            .await
            .expect("file token")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_picture_ids_require_the_upstream_absolute_path_shape() {
        let id = Uuid::from_u128(7);
        assert_eq!(
            core_picture_file_id(&format!(
                "https://cdn.example/file/core-picture/{id}?token=old"
            )),
            Some(id)
        );
        assert_eq!(
            core_picture_file_id(&format!("/file/core-picture/{id}")),
            None
        );
        assert_eq!(
            core_picture_file_id(&format!("https://cdn.example/file/files-field/{id}")),
            None
        );
        assert_eq!(core_picture_file_id("not a url"), None);
    }

    #[test]
    fn relative_paths_reject_traversal_and_platform_prefixes() {
        assert_eq!(
            safe_relative_path("core-picture/avatar.png"),
            Some(PathBuf::from("core-picture/avatar.png"))
        );
        for value in [
            "",
            "/core-picture/avatar.png",
            "../avatar.png",
            "core-picture/../avatar.png",
            "core-picture\\avatar.png",
            "core-picture/\0avatar.png",
        ] {
            assert_eq!(safe_relative_path(value), None, "{value:?}");
        }
    }

    #[test]
    fn byte_ranges_cover_full_open_suffix_and_invalid_forms() {
        assert_eq!(parse_byte_range(None, 10), ByteRange::Full);
        assert_eq!(parse_byte_range(Some("items=1-2"), 10), ByteRange::Full);
        assert_eq!(parse_byte_range(Some("bytes=1-2,4-5"), 10), ByteRange::Full);
        assert_eq!(parse_byte_range(Some("bytes=4-2"), 10), ByteRange::Full);
        assert_eq!(
            parse_byte_range(Some("bytes=2-4"), 10),
            ByteRange::Partial {
                start: 2,
                length: 3
            }
        );
        assert_eq!(
            parse_byte_range(Some("bytes=4-"), 10),
            ByteRange::Partial {
                start: 4,
                length: 6
            }
        );
        assert_eq!(
            parse_byte_range(Some("bytes=-3"), 10),
            ByteRange::Partial {
                start: 7,
                length: 3
            }
        );
        assert_eq!(
            parse_byte_range(Some("bytes=-0"), 10),
            ByteRange::Unsatisfiable
        );
        assert_eq!(
            parse_byte_range(Some("bytes=10-"), 10),
            ByteRange::Unsatisfiable
        );
        assert_eq!(
            parse_byte_range(Some("bytes=0-1"), 0),
            ByteRange::Unsatisfiable
        );
    }

    #[test]
    fn content_disposition_matches_the_upstream_inline_allowlist() {
        assert_eq!(
            content_disposition("image/png"),
            HeaderValue::from_static("inline")
        );
        assert_eq!(
            content_disposition("image/svg+xml"),
            HeaderValue::from_static("attachment")
        );
        assert_eq!(
            content_disposition("audio/mp3"),
            HeaderValue::from_static("attachment")
        );
        assert_eq!(
            content_disposition("application/pdf"),
            HeaderValue::from_static("inline")
        );
    }

    #[test]
    fn core_pictures_enforce_the_upstream_mime_allowlist() {
        for mime_type in [
            "image/gif",
            "image/jpeg",
            "image/png",
            "image/svg+xml",
            "image/webp",
        ] {
            assert!(FileFolder::CorePicture.allows_mime_type(mime_type));
        }
        assert!(!FileFolder::CorePicture.allows_mime_type("text/html"));
        assert!(FileFolder::FilesField.allows_mime_type("text/html"));
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn open_descriptor_verification_rejects_files_outside_storage() {
        let root = std::env::temp_dir().join(format!("nudge-file-fd-{}", Uuid::new_v4()));
        let workspace_id = Uuid::from_u128(3);
        let application_id = Uuid::from_u128(4);
        let directory = root
            .join(workspace_id.to_string())
            .join(application_id.to_string());
        std::fs::create_dir_all(&directory).expect("creating descriptor fixture");
        std::fs::write(directory.join("avatar.png"), b"avatar")
            .expect("writing descriptor fixture");
        let inside = File::open(directory.join("avatar.png"))
            .await
            .expect("opening inside file");
        let outside = File::open("/etc/passwd")
            .await
            .expect("opening outside file");
        assert!(opened_file_within_storage(
            &inside,
            &root,
            workspace_id,
            application_id
        ));
        assert!(!opened_file_within_storage(
            &outside,
            &root,
            workspace_id,
            application_id
        ));
        std::fs::remove_dir_all(root).expect("removing descriptor fixture");
    }

    #[cfg(unix)]
    #[test]
    fn local_resolution_rejects_symlink_escapes() {
        let root = std::env::temp_dir().join(format!("nudge-file-test-{}", Uuid::new_v4()));
        let workspace_id = Uuid::from_u128(1);
        let application_id = Uuid::from_u128(2);
        let workspace = root.join(workspace_id.to_string());
        let application = workspace.join(application_id.to_string());
        std::fs::create_dir_all(&application).expect("creating fixture directories");
        std::fs::write(application.join("avatar.png"), b"avatar").expect("writing fixture");
        std::os::unix::fs::symlink("/etc/passwd", application.join("escape.png"))
            .expect("creating escape symlink");
        assert_eq!(
            resolve_local_path(&root, workspace_id, application_id, Path::new("avatar.png")),
            Some(
                application
                    .canonicalize()
                    .expect("canonical application")
                    .join("avatar.png")
            )
        );
        assert_eq!(
            resolve_local_path(&root, workspace_id, application_id, Path::new("escape.png")),
            None
        );
        std::fs::remove_dir_all(root).expect("removing fixture directories");
    }
}
