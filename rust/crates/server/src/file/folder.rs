use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFolder {
    RecordExport,
    CorePicture,
    AgentChat,
    BuiltLogicFunction,
    BuiltFrontComponent,
    PublicAsset,
    Source,
    FilesField,
    Dependencies,
    Workflow,
    EmailAttachment,
    EmailImage,
    AppTarball,
    GeneratedSdkClient,
    Dpa,
    WorkspaceDataExport,
}

pub const ANY_MIME_TYPE: Option<&'static [&'static str]> = None;

pub const CORE_PICTURE_MIME_TYPES: &[&str] = &[
    "image/gif",
    "image/jpeg",
    "image/png",
    "image/svg+xml",
    "image/webp",
];

pub const EMAIL_IMAGE_MIME_TYPES: &[&str] = &["image/gif", "image/jpeg", "image/png", "image/webp"];

const IMMUTABLE_FILE_CACHE_CONTROL: &str = "private, max-age=86400, immutable";
const PUBLIC_ASSET_CACHE_CONTROL: &str = "public, max-age=3600";
const NO_STORE_CACHE_CONTROL: &str = "private, no-store";

const SOURCE_EXTENSIONS: &[&str] = &[
    ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".mts", ".cts", ".json", ".css", ".png", ".jpg",
    ".jpeg", ".gif", ".svg", ".webp", ".avif", ".ico", ".woff", ".woff2", ".ttf", ".eot", ".otf",
];
const BUILT_EXTENSIONS: &[&str] = &[".mjs"];
const DEPENDENCIES_EXTENSIONS: &[&str] = &[".json", ".lock", ".gz"];

impl FileFolder {
    pub const ALL: [FileFolder; 16] = [
        FileFolder::RecordExport,
        FileFolder::CorePicture,
        FileFolder::AgentChat,
        FileFolder::BuiltLogicFunction,
        FileFolder::BuiltFrontComponent,
        FileFolder::PublicAsset,
        FileFolder::Source,
        FileFolder::FilesField,
        FileFolder::Dependencies,
        FileFolder::Workflow,
        FileFolder::EmailAttachment,
        FileFolder::EmailImage,
        FileFolder::AppTarball,
        FileFolder::GeneratedSdkClient,
        FileFolder::Dpa,
        FileFolder::WorkspaceDataExport,
    ];

    pub const DIRECT_UPLOAD: [FileFolder; 7] = [
        FileFolder::FilesField,
        FileFolder::Workflow,
        FileFolder::EmailAttachment,
        FileFolder::AgentChat,
        FileFolder::EmailImage,
        FileFolder::AppTarball,
        FileFolder::CorePicture,
    ];

    pub const DEDICATED_COMPLETION: [FileFolder; 2] =
        [FileFolder::AppTarball, FileFolder::CorePicture];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::RecordExport => "record-export",
            Self::CorePicture => "core-picture",
            Self::AgentChat => "agent-chat",
            Self::BuiltLogicFunction => "built-logic-function",
            Self::BuiltFrontComponent => "built-front-component",
            Self::PublicAsset => "public-asset",
            Self::Source => "source",
            Self::FilesField => "files-field",
            Self::Dependencies => "dependencies",
            Self::Workflow => "workflow",
            Self::EmailAttachment => "email-attachment",
            Self::EmailImage => "email-image",
            Self::AppTarball => "app-tarball",
            Self::GeneratedSdkClient => "generated-sdk-client",
            Self::Dpa => "dpa",
            Self::WorkspaceDataExport => "workspace-data-export",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|folder| folder.as_str() == value)
    }

    pub fn pattern(self) -> String {
        format!("{}/%", self.as_str())
    }

    pub fn from_path(path: &str) -> Option<Self> {
        Self::parse(path.split('/').next().unwrap_or_default())
    }

    pub fn ignores_expiration_token(self) -> bool {
        matches!(
            self,
            Self::CorePicture | Self::PublicAsset | Self::EmailImage
        )
    }

    pub fn enforces_expiration(self) -> bool {
        !self.ignores_expiration_token()
    }

    pub fn cache_control(self) -> Option<&'static str> {
        match self {
            Self::RecordExport | Self::WorkspaceDataExport => Some(NO_STORE_CACHE_CONTROL),
            Self::CorePicture
            | Self::AgentChat
            | Self::BuiltFrontComponent
            | Self::FilesField
            | Self::Workflow
            | Self::EmailAttachment
            | Self::EmailImage
            | Self::Dpa => Some(IMMUTABLE_FILE_CACHE_CONTROL),
            Self::PublicAsset => Some(PUBLIC_ASSET_CACHE_CONTROL),
            Self::BuiltLogicFunction
            | Self::Source
            | Self::Dependencies
            | Self::AppTarball
            | Self::GeneratedSdkClient => None,
        }
    }

    pub fn allowed_mime_types(self) -> Option<&'static [&'static str]> {
        match self {
            Self::RecordExport => Some(&["text/csv"]),
            Self::CorePicture => Some(CORE_PICTURE_MIME_TYPES),
            Self::EmailImage => Some(EMAIL_IMAGE_MIME_TYPES),
            Self::AppTarball => Some(&["application/gzip"]),
            Self::WorkspaceDataExport => Some(&["application/zip", "text/csv", "application/json"]),
            Self::AgentChat
            | Self::BuiltLogicFunction
            | Self::BuiltFrontComponent
            | Self::PublicAsset
            | Self::Source
            | Self::FilesField
            | Self::Dependencies
            | Self::Workflow
            | Self::EmailAttachment
            | Self::GeneratedSdkClient
            | Self::Dpa => ANY_MIME_TYPE,
        }
    }

    pub fn allows_mime_type(self, mime_type: &str) -> bool {
        match self.allowed_mime_types() {
            None => true,
            Some(allowed) => allowed.contains(&mime_type),
        }
    }

    pub fn allowed_extensions(self) -> Option<&'static [&'static str]> {
        match self {
            Self::BuiltLogicFunction | Self::BuiltFrontComponent => Some(BUILT_EXTENSIONS),
            Self::Source => Some(SOURCE_EXTENSIONS),
            Self::Dependencies => Some(DEPENDENCIES_EXTENSIONS),
            _ => None,
        }
    }

    pub fn max_file_size_bytes(self) -> u64 {
        match self {
            Self::CorePicture => 10 * 1024 * 1024,
            _ => 1024 * 1024 * 1024,
        }
    }
}

impl fmt::Display for FileFolder {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}
