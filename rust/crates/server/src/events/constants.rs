pub const EVENT_STREAM_TTL_MS: u64 = 30 * 60 * 1_000;

pub const APPLICATION_KEEPALIVE_INTERVAL_MS: u64 = 30 * 1_000;

pub const WORKFLOWS_PERMISSION_FLAG: &str = "WORKFLOWS";

const REQUIRED_PERMISSION_FLAG_BY_BROADCAST_ENTITY_NAME: &[(&str, &str)] = &[
    ("workflow", WORKFLOWS_PERMISSION_FLAG),
    ("workflowVersion", WORKFLOWS_PERMISSION_FLAG),
];

const WORKFLOW_BROADCAST_PROPERTY_KEYS: &[&str] = &["id"];
const WORKFLOW_VERSION_BROADCAST_PROPERTY_KEYS: &[&str] = &["id", "coreWorkflowId"];

const BROADCAST_PROPERTY_KEYS_BY_GATED_ENTITY_NAME: &[(&str, &[&str])] = &[
    ("workflow", WORKFLOW_BROADCAST_PROPERTY_KEYS),
    ("workflowVersion", WORKFLOW_VERSION_BROADCAST_PROPERTY_KEYS),
];

pub const VIEW_CHILD_METADATA_NAMES: &[&str] = &[
    "viewField",
    "viewFieldGroup",
    "viewFilter",
    "viewFilterGroup",
    "viewGroup",
    "viewSort",
];

pub const TOOL_PERMISSION_FLAGS: &[&str] = &[
    "AI",
    "VIEWS",
    "UPLOAD_FILE",
    "DOWNLOAD_FILE",
    "SEND_EMAIL_TOOL",
    "CREATE_CALENDAR_EVENT_TOOL",
    "HTTP_REQUEST_TOOL",
    "IMPORT_CSV",
    "EXPORT_CSV",
    "CONNECTED_ACCOUNTS",
    "PROFILE_INFORMATION",
    "CODE_INTERPRETER_TOOL",
];

pub fn is_tool_permission_flag(permission_flag: &str) -> bool {
    TOOL_PERMISSION_FLAGS.contains(&permission_flag)
}

pub fn required_permission_flag_for_entity(entity_name: &str) -> Option<&'static str> {
    REQUIRED_PERMISSION_FLAG_BY_BROADCAST_ENTITY_NAME
        .iter()
        .find(|(name, _)| *name == entity_name)
        .map(|(_, flag)| *flag)
}

pub fn broadcast_property_keys_for_gated_entity(
    entity_name: &str,
) -> Option<&'static [&'static str]> {
    BROADCAST_PROPERTY_KEYS_BY_GATED_ENTITY_NAME
        .iter()
        .find(|(name, _)| *name == entity_name)
        .map(|(_, keys)| *keys)
}

pub fn is_view_child_metadata_name(metadata_name: &str) -> bool {
    VIEW_CHILD_METADATA_NAMES.contains(&metadata_name)
}

pub fn ttl_refresh_interval_ms() -> u64 {
    EVENT_STREAM_TTL_MS / 5
}
