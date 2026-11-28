use std::collections::HashMap;

use crate::file::validation::file_extension;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInfo {
    pub mime_type: String,
    pub ext: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MimeError {
    ContentMismatch { ext: String, expected: String },
}

impl MimeError {
    pub fn message(&self) -> String {
        match self {
            Self::ContentMismatch { ext, expected } => format!(
                "File content does not match its extension. The file has extension '{ext}' (expected mime type: {expected}), but the file content could not be detected as this type. The file may be corrupted, have the wrong extension, or be a security risk."
            ),
        }
    }
}

pub fn twenty_mime_policy(ext: &str) -> Option<&'static str> {
    match ext {
        "ts" | "tsx" => Some("application/typescript"),
        _ => None,
    }
}

pub fn mime_from_extension(ext: &str) -> Option<&'static str> {
    let table = extension_table();

    table.get(ext).copied()
}

pub fn extract_file_info(buffer: &[u8], filename: &str) -> Result<FileInfo, MimeError> {
    let declared_ext = file_extension(filename);

    if let Some((detected_ext, detected_mime)) = sniff(buffer) {
        return Ok(FileInfo {
            mime_type: detected_mime.to_string(),
            ext: detected_ext.to_string(),
        });
    }

    let ext = declared_ext;
    let mut mime_type = String::from("application/octet-stream");

    if !ext.is_empty() {
        if let Some(policy_mime) = twenty_mime_policy(&ext) {
            return Ok(FileInfo {
                mime_type: policy_mime.to_string(),
                ext,
            });
        }

        let from_extension = mime_from_extension(&ext);

        if let Some(candidate) = from_extension {
            if detectable_mimes().contains(&candidate) {
                return Err(MimeError::ContentMismatch {
                    ext,
                    expected: candidate.to_string(),
                });
            }

            mime_type = candidate.to_string();
        }
    }

    Ok(FileInfo { mime_type, ext })
}

pub fn sniff(buffer: &[u8]) -> Option<(&'static str, &'static str)> {
    if starts_with(buffer, b"\x89PNG\r\n\x1a\n") {
        return Some(("png", "image/png"));
    }

    if starts_with(buffer, b"\xff\xd8\xff") {
        return Some(("jpg", "image/jpeg"));
    }

    if starts_with(buffer, b"GIF87a") || starts_with(buffer, b"GIF89a") {
        return Some(("gif", "image/gif"));
    }

    if starts_with(buffer, b"BM") && buffer.len() >= 18 {
        return Some(("bmp", "image/bmp"));
    }

    if starts_with(buffer, b"RIFF") && slice_at(buffer, 8, 4) == Some(b"WEBP") {
        return Some(("webp", "image/webp"));
    }

    if starts_with(buffer, b"RIFF") && slice_at(buffer, 8, 4) == Some(b"AVI ") {
        return Some(("avi", "video/x-msvideo"));
    }

    if slice_at(buffer, 0, 4) == Some(b"\x1aE\xdf\xa3") {
        return Some(("webm", "video/webm"));
    }

    if starts_with(buffer, b"OggS") {
        return Some(("ogg", "audio/ogg"));
    }

    if starts_with(buffer, b"fLaC") {
        return Some(("flac", "audio/flac"));
    }

    if starts_with(buffer, b"ID3") {
        return Some(("mp3", "audio/mpeg"));
    }

    if buffer.first() == Some(&0xff) && buffer.get(1).is_some_and(|byte| byte & 0xe0 == 0xe0) {
        return Some(("mp3", "audio/mpeg"));
    }

    if starts_with(buffer, b"%PDF") {
        return Some(("pdf", "application/pdf"));
    }

    if starts_with(buffer, b"PK\x03\x04") {
        if zip_entry_name(buffer).is_some_and(|name| name.starts_with("mimetype")) {
            if let Some(offset) = find(buffer, b"application/vnd.oasis.opendocument") {
                let _ = offset;
                return Some(("odt", "application/vnd.oasis.opendocument.text"));
            }
        }

        return Some(("zip", "application/zip"));
    }

    if starts_with(buffer, b"\x1f\x8b") {
        return Some(("gz", "application/gzip"));
    }

    if starts_with(buffer, b"7z\xbc\xaf\x27\x1c") {
        return Some(("7z", "application/x-7z-compressed"));
    }

    if starts_with(buffer, b"Rar!\x1a\x07") {
        return Some(("rar", "application/vnd.rar"));
    }

    if starts_with(buffer, b"\x7fELF") {
        return Some(("exe", "application/x-executable"));
    }

    if starts_with(buffer, b"\x00\x00\x01\x00") && buffer.len() > 6 {
        return Some(("ico", "image/x-icon"));
    }

    if slice_at(buffer, 0, 4) == Some(b"II*\x00") || slice_at(buffer, 0, 4) == Some(b"MM\x00*") {
        return Some(("tif", "image/tiff"));
    }

    if starts_with(buffer, b"RIFF") && slice_at(buffer, 8, 4) == Some(b"WAVE") {
        return Some(("wav", "audio/wav"));
    }

    if slice_at(buffer, 4, 4) == Some(b"ftyp") {
        let brand = slice_at(buffer, 8, 4).unwrap_or_default();
        let alignment = slice_at(buffer, 20, 8).unwrap_or_default();

        if alignment == b"ftypavif" || brand == b"avif" || brand == b"avis" {
            return Some(("avif", "image/avif"));
        }

        if alignment == b"ftypheic" {
            return Some(("heic", "image/heif"));
        }

        if brand == b"qt  " {
            return Some(("mov", "video/quicktime"));
        }

        if matches!(
            brand,
            b"iso " | b"iso2" | b"mp41" | b"mp42" | b"M4V " | b"dash" | b"mmp4"
        ) {
            return Some(("mp4", "video/mp4"));
        }

        if matches!(brand, b"M4A " | b"M4B " | b"M4P " | b"M4VN") {
            return Some(("m4a", "audio/mp4"));
        }

        return Some(("mp4", "video/mp4"));
    }

    if let Some(markup) = text_markup(buffer) {
        return Some(markup);
    }

    None
}

fn text_markup(buffer: &[u8]) -> Option<(&'static str, &'static str)> {
    let head = &buffer[..buffer.len().min(4096)];
    let trimmed = trim_ascii(head);

    if trimmed.starts_with(b"<?xml") {
        if contains_ci(trimmed, b"<svg") {
            return Some(("svg", "image/svg+xml"));
        }

        return Some(("xml", "text/xml"));
    }

    if trimmed.starts_with(b"<svg") {
        return Some(("svg", "image/svg+xml"));
    }

    if trimmed.starts_with(b"<!doctype html") || trimmed.starts_with(b"<html") {
        return Some(("html", "text/html"));
    }

    if trimmed.starts_with(b"{") || trimmed.starts_with(b"[") {
        return Some(("json", "application/json"));
    }

    None
}

fn detectable_mimes() -> Vec<&'static str> {
    let pairs = [
        ("png", "image/png"),
        ("jpg", "image/jpeg"),
        ("gif", "image/gif"),
        ("bmp", "image/bmp"),
        ("webp", "image/webp"),
        ("avi", "video/x-msvideo"),
        ("webm", "video/webm"),
        ("ogg", "audio/ogg"),
        ("flac", "audio/flac"),
        ("mp3", "audio/mpeg"),
        ("pdf", "application/pdf"),
        ("zip", "application/zip"),
        ("gz", "application/gzip"),
        ("7z", "application/x-7z-compressed"),
        ("rar", "application/vnd.rar"),
        ("exe", "application/x-executable"),
        ("ico", "image/x-icon"),
        ("tif", "image/tiff"),
        ("wav", "audio/wav"),
        ("avif", "image/avif"),
        ("heic", "image/heif"),
        ("mov", "video/quicktime"),
        ("mp4", "video/mp4"),
        ("m4a", "audio/mp4"),
        ("svg", "image/svg+xml"),
        ("xml", "text/xml"),
        ("html", "text/html"),
        ("json", "application/json"),
    ];

    pairs.into_iter().map(|(_, mime)| mime).collect()
}

fn extension_table() -> HashMap<&'static str, &'static str> {
    [
        ("png", "image/png"),
        ("jpg", "image/jpeg"),
        ("jpeg", "image/jpeg"),
        ("gif", "image/gif"),
        ("webp", "image/webp"),
        ("avif", "image/avif"),
        ("bmp", "image/bmp"),
        ("tif", "image/tiff"),
        ("tiff", "image/tiff"),
        ("ico", "image/x-icon"),
        ("svg", "image/svg+xml"),
        ("pdf", "application/pdf"),
        ("zip", "application/zip"),
        ("gz", "application/gzip"),
        ("json", "application/json"),
        ("txt", "text/plain"),
        ("csv", "text/csv"),
        ("md", "text/markdown"),
        ("html", "text/html"),
        ("htm", "text/html"),
        ("xml", "text/xml"),
        ("mp3", "audio/mpeg"),
        ("wav", "audio/wav"),
        ("ogg", "audio/ogg"),
        ("mp4", "video/mp4"),
        ("webm", "video/webm"),
        ("eml", "message/rfc822"),
        ("mjs", "text/javascript"),
        ("js", "text/javascript"),
        ("css", "text/css"),
        ("lock", "text/plain"),
    ]
    .into_iter()
    .collect()
}

fn starts_with(buffer: &[u8], prefix: &[u8]) -> bool {
    buffer.len() >= prefix.len() && &buffer[..prefix.len()] == prefix
}

fn slice_at(buffer: &[u8], offset: usize, length: usize) -> Option<&[u8]> {
    buffer.get(offset..offset + length)
}

fn find(buffer: &[u8], needle: &[u8]) -> Option<usize> {
    buffer
        .windows(needle.len())
        .position(|window| window == needle)
}

fn zip_entry_name(buffer: &[u8]) -> Option<&str> {
    let name_length = u16::from_le_bytes([*buffer.get(26)?, *buffer.get(27)?]) as usize;
    let raw = buffer.get(30..30 + name_length)?;

    std::str::from_utf8(raw).ok()
}

fn trim_ascii(buffer: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = buffer.len();

    while start < end
        && matches!(
            buffer[start],
            b' ' | b'\t' | b'\r' | b'\n' | 0xef | 0xbb | 0xbf
        )
    {
        start += 1;
    }

    while end > start && matches!(buffer[end - 1], b' ' | b'\t' | b'\r' | b'\n') {
        end -= 1;
    }

    &buffer[start..end]
}

fn contains_ci(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|window| {
        window
            .iter()
            .zip(needle.iter())
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
    })
}
