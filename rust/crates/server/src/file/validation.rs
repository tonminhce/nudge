use uuid::Uuid;

use crate::file::folder::FileFolder;

pub const PENDING_UPLOAD_PATH_PREFIX: &str = ".pending";
pub const MAX_PATH_LENGTH: usize = 1024;
pub const MAX_SEGMENT_LENGTH: usize = 255;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathValidation {
    pub is_valid: bool,
    pub error: Option<String>,
}

impl PathValidation {
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            error: None,
        }
    }

    pub fn invalid(error: impl Into<String>) -> Self {
        Self {
            is_valid: false,
            error: Some(error.into()),
        }
    }
}

pub fn assert_storage_path_is_safe(storage_path: &str) -> Result<(), String> {
    if storage_path.contains('\0') {
        return Err(String::from("Invalid storage path: contains null bytes"));
    }

    if starts_with_absolute(storage_path) {
        return Err(String::from(
            "Invalid storage path: absolute path not allowed",
        ));
    }

    if normalized_segments(storage_path).contains(&"..") {
        return Err(String::from(
            "Invalid storage path: path traversal detected",
        ));
    }

    Ok(())
}

pub fn validate_safe_relative_path(resource_path: &str) -> PathValidation {
    if resource_path.is_empty() {
        return PathValidation::invalid("Resource path must not be empty");
    }

    if resource_path.contains('\0') {
        return PathValidation::invalid("Resource path contains null bytes");
    }

    if starts_with_absolute(resource_path) {
        return PathValidation::invalid("Resource path must be relative, not absolute");
    }

    if resource_path.contains('\\') {
        return PathValidation::invalid("Resource path must not contain backslashes");
    }

    if normalized_segments(resource_path).contains(&"..") {
        return PathValidation::invalid("Resource path must not contain path traversal (..)");
    }

    PathValidation::valid()
}

pub fn validate_path_segments_safety(resource_path: &str) -> PathValidation {
    if resource_path.len() > MAX_PATH_LENGTH {
        return PathValidation::invalid(format!(
            "Resource path exceeds maximum length of {MAX_PATH_LENGTH} characters"
        ));
    }

    if resource_path.contains("//") || resource_path.ends_with('/') {
        return PathValidation::invalid(
            "Resource path must not contain empty segments or trailing slashes",
        );
    }

    for segment in resource_path.split('/') {
        if segment.len() > MAX_SEGMENT_LENGTH {
            return PathValidation::invalid(
                "A path segment exceeds the maximum length of 255 characters",
            );
        }

        if !segment.is_empty()
            && segment.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
            })
        {
            continue;
        }

        return PathValidation::invalid(
            "A path segment contains invalid characters. Only alphanumeric, dots, dashes and underscores are allowed",
        );
    }

    PathValidation::valid()
}

pub fn validate_file_extension(resource_path: &str, folder: FileFolder) -> PathValidation {
    let Some(allowed) = folder.allowed_extensions() else {
        return PathValidation::valid();
    };

    let extension = extname(resource_path).to_lowercase();

    if allowed.contains(&extension.as_str()) {
        return PathValidation::valid();
    }

    PathValidation::invalid(format!(
        "Invalid file extension. Allowed extensions: {}",
        allowed.join(", ")
    ))
}

pub fn validate_file_path(resource_path: &str, folder: FileFolder) -> PathValidation {
    let safe = validate_safe_relative_path(resource_path);

    if !safe.is_valid {
        return safe;
    }

    let segments = validate_path_segments_safety(resource_path);

    if !segments.is_valid {
        return segments;
    }

    let filename = resource_path.rsplit('/').next().unwrap_or_default();

    if !filename.contains('.') {
        return PathValidation::invalid("Filename must have an extension");
    }

    validate_file_extension(resource_path, folder)
}

pub fn build_pending_upload_resource_path(file_id: Uuid, resource_path: &str) -> String {
    format!("{PENDING_UPLOAD_PATH_PREFIX}/{file_id}/{resource_path}")
}

pub fn file_extension(filename: &str) -> String {
    let mut parts = filename.split('.').collect::<Vec<_>>();

    if parts.len() > 1 {
        String::from(parts.pop().unwrap_or_default())
    } else {
        String::new()
    }
}

pub fn remove_file_folder_from_file_entity_path(path: &str) -> Result<String, String> {
    let Some(folder) = FileFolder::from_path(path) else {
        return Err(format!(
            "File folder {} is not allowed",
            path.split('/').next().unwrap_or_default()
        ));
    };

    Ok(path.replacen(&format!("{}/", folder.as_str()), "", 1))
}

pub fn extname(path: &str) -> String {
    let filename = path.rsplit('/').next().unwrap_or(path);
    let Some((_, extension)) = filename.rsplit_once('.') else {
        return String::new();
    };

    if extension.is_empty() || filename.rfind('.') == Some(0) {
        return String::new();
    }

    format!(".{extension}")
}

fn starts_with_absolute(path: &str) -> bool {
    path.starts_with('/')
        || (path.as_bytes().get(1) == Some(&b':') && path.as_bytes()[0].is_ascii_alphabetic())
}

fn normalized_segments(path: &str) -> Vec<&str> {
    let mut segments: Vec<&str> = Vec::new();

    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if segments.last().is_some_and(|value| *value != "..") {
                    segments.pop();
                } else {
                    segments.push("..");
                }
            }
            value => segments.push(value),
        }
    }

    segments
}
