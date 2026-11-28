use std::collections::HashMap;
use std::collections::HashSet;

use serde_json::Map;
use serde_json::Value;
use uuid::Uuid;

use super::compile::compile_named_parameters;
use super::repository::quote_identifier;
use super::runner::base::execute_json_rows;
use super::runner::base::ObjectModel;
use super::runner::RunnerError;
use super::runner::RunnerSchema;
use crate::metadata::flat::FieldMetadataType;

pub const FIELD_RESTRICTED_ADDITIONAL_PERMISSIONS_REQUIRED: &str =
    "FIELD_RESTRICTED_ADDITIONAL_PERMISSIONS_REQUIRED";

const MESSAGE_OBJECT: &str = "message";
const ATTACHMENT_OBJECT: &str = "attachment";
const CALENDAR_EVENT_OBJECT: &str = "calendarEvent";
const MESSAGE_CHANNEL_ASSOCIATION_OBJECT: &str = "messageChannelMessageAssociation";
const CALENDAR_CHANNEL_ASSOCIATION_OBJECT: &str = "calendarChannelEventAssociation";

const MESSAGE_CHANNEL_TABLE: &str = "messageChannel";
const CALENDAR_CHANNEL_TABLE: &str = "calendarChannel";
const CONNECTED_ACCOUNT_TABLE: &str = "connectedAccount";

const RESTRICTED_MESSAGE_FIELDS: [&str; 3] = ["subject", "text", "attachments"];
const RESTRICTED_ATTACHMENT_FIELDS: [&str; 3] = ["name", "fullPath", "file"];
const RESTRICTED_CALENDAR_EVENT_FIELDS: [&str; 2] = ["title", "description"];

const ATTACHMENT_TARGET_FIELD: &str = "targetMessage";
const SHARE_EVERYTHING_VISIBILITY: &str = "SHARE_EVERYTHING";
const SUBJECT_VISIBILITY: &str = "SUBJECT";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageChannelVisibility {
    Metadata,
    Subject,
    ShareEverything,
}

impl MessageChannelVisibility {
    pub fn parse(raw: &str) -> Self {
        match raw {
            SHARE_EVERYTHING_VISIBILITY => MessageChannelVisibility::ShareEverything,
            SUBJECT_VISIBILITY => MessageChannelVisibility::Subject,
            _ => MessageChannelVisibility::Metadata,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarChannelVisibility {
    Metadata,
    ShareEverything,
}

impl CalendarChannelVisibility {
    pub fn parse(raw: &str) -> Self {
        if raw == SHARE_EVERYTHING_VISIBILITY {
            CalendarChannelVisibility::ShareEverything
        } else {
            CalendarChannelVisibility::Metadata
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelViewer {
    User { user_workspace_id: Uuid },
    Application { application_id: Uuid },
    NoOwnership,
}

impl ChannelViewer {
    pub fn from_credentials(user_workspace_id: Option<Uuid>, application_id: Option<Uuid>) -> Self {
        if let Some(user_workspace_id) = user_workspace_id {
            return ChannelViewer::User { user_workspace_id };
        }

        if let Some(application_id) = application_id {
            return ChannelViewer::Application { application_id };
        }

        ChannelViewer::NoOwnership
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChannelOwner {
    pub user_workspace_id: Option<Uuid>,
    pub application_id: Option<Uuid>,
}

impl ChannelOwner {
    pub fn owns_messages(&self, viewer: ChannelViewer) -> bool {
        match viewer {
            ChannelViewer::User { user_workspace_id } => {
                self.user_workspace_id == Some(user_workspace_id)
            }
            ChannelViewer::Application { application_id } => {
                self.application_id == Some(application_id)
            }
            ChannelViewer::NoOwnership => false,
        }
    }

    pub fn owns_calendar_events(&self, viewer: ChannelViewer) -> bool {
        match viewer {
            ChannelViewer::User { user_workspace_id } => {
                self.user_workspace_id == Some(user_workspace_id)
            }
            ChannelViewer::Application { .. } | ChannelViewer::NoOwnership => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelRow {
    pub id: Uuid,
    pub visibility: String,
    pub owner: ChannelOwner,
}

impl ChannelRow {
    pub fn message_visibility(&self) -> MessageChannelVisibility {
        MessageChannelVisibility::parse(&self.visibility)
    }

    pub fn calendar_visibility(&self) -> CalendarChannelVisibility {
        CalendarChannelVisibility::parse(&self.visibility)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MessageVisibility {
    pub fully_visible: HashSet<Uuid>,
    pub subject_visible: HashSet<Uuid>,
}

pub fn resolve_message_visibility(
    associations: &[(Uuid, Uuid)],
    channels: &[ChannelRow],
    viewer: ChannelViewer,
) -> MessageVisibility {
    let channel_by_id = channels
        .iter()
        .map(|channel| (channel.id, channel))
        .collect::<HashMap<_, _>>();
    let mut visibility = MessageVisibility::default();

    for (message_id, channel_id) in associations {
        let Some(channel) = channel_by_id.get(channel_id) else {
            continue;
        };

        if channel.message_visibility() == MessageChannelVisibility::ShareEverything
            || channel.owner.owns_messages(viewer)
        {
            visibility.fully_visible.insert(*message_id);
            visibility.subject_visible.remove(message_id);
        } else if channel.message_visibility() == MessageChannelVisibility::Subject
            && !visibility.fully_visible.contains(message_id)
        {
            visibility.subject_visible.insert(*message_id);
        }
    }

    visibility
}

pub fn resolve_visible_calendar_event_ids(
    associations: &[(Uuid, Uuid)],
    channels: &[ChannelRow],
    viewer: ChannelViewer,
) -> HashSet<Uuid> {
    let visible_channel_ids = channels
        .iter()
        .filter(|channel| {
            channel.calendar_visibility() == CalendarChannelVisibility::ShareEverything
                || channel.owner.owns_calendar_events(viewer)
        })
        .map(|channel| channel.id)
        .collect::<HashSet<_>>();

    associations
        .iter()
        .filter(|(_, channel_id)| visible_channel_ids.contains(channel_id))
        .map(|(event_id, _)| *event_id)
        .collect()
}

pub fn restricted_attachment_ids(
    targets: &[(Uuid, Option<Uuid>)],
    fully_visible_message_ids: &HashSet<Uuid>,
) -> HashSet<Uuid> {
    targets
        .iter()
        .filter(|(_, target_message_id)| {
            target_message_id
                .as_ref()
                .is_some_and(|target| !fully_visible_message_ids.contains(target))
        })
        .map(|(attachment_id, _)| *attachment_id)
        .collect()
}

#[derive(Debug, Clone)]
struct AttachmentTarget {
    projected: Option<Uuid>,
    unprojected: bool,
}

#[derive(Debug, Default)]
struct VisibilityScan {
    message_ids: HashSet<Uuid>,
    calendar_event_ids: HashSet<Uuid>,
    attachment_targets: Vec<(Uuid, AttachmentTarget)>,
}

impl VisibilityScan {
    fn is_empty(&self) -> bool {
        self.message_ids.is_empty()
            && self.calendar_event_ids.is_empty()
            && self.attachment_targets.is_empty()
    }
}

#[derive(Debug, Default)]
struct VisibilityDecisions {
    message_visibility: MessageVisibility,
    restricted_attachments: HashSet<Uuid>,
    restricted_calendar_events: HashSet<Uuid>,
}

fn quoted_identifier(identifier: &str) -> Result<String, RunnerError> {
    quote_identifier(identifier).map_err(RunnerError::from_repository)
}

fn record_id(record: &Value) -> Option<Uuid> {
    Uuid::parse_str(record.get("id")?.as_str()?).ok()
}

fn holds_any_field(record: &Value, field_names: &[&str]) -> bool {
    record
        .as_object()
        .is_some_and(|entries| field_names.iter().any(|name| entries.contains_key(*name)))
}

fn relation_targets(model: &ObjectModel, schema: &RunnerSchema) -> Vec<(String, String)> {
    let mut children = model
        .shape
        .relation_shape_by_field_name
        .iter()
        .filter_map(|(field_name, relation)| {
            let target_id = relation.target_object_metadata_id?;
            let target = schema.object_by_id(&target_id).ok()?;

            Some((field_name.clone(), target.object.name_singular.clone()))
        })
        .collect::<Vec<_>>();
    children.sort_by(|left, right| left.0.cmp(&right.0));

    children
}

fn visit_nested_records<F>(
    record: &mut Value,
    object_name: &str,
    schema: &RunnerSchema,
    visit: &mut F,
) where
    F: FnMut(&mut Value, &str),
{
    let Ok(model) = schema.object(object_name) else {
        return;
    };
    let children = relation_targets(model, schema);

    for (field_name, child_name) in children {
        let Some(child) = record.get_mut(field_name.as_str()) else {
            continue;
        };

        match child {
            Value::Object(_) => {
                visit(child, &child_name);
                visit_nested_records(child, &child_name, schema, visit);
            }
            Value::Array(items) => {
                for item in items.iter_mut() {
                    if !item.is_object() {
                        continue;
                    }
                    visit(item, &child_name);
                    visit_nested_records(item, &child_name, schema, visit);
                }
            }
            _ => {}
        }
    }
}

fn attachment_target_in_record(record: &Value) -> AttachmentTarget {
    for key in [
        format!("{ATTACHMENT_TARGET_FIELD}Id"),
        String::from(ATTACHMENT_TARGET_FIELD),
    ] {
        let Some(value) = record.get(key.as_str()) else {
            continue;
        };

        return match value {
            Value::Null => AttachmentTarget {
                projected: None,
                unprojected: false,
            },
            Value::Object(object) => AttachmentTarget {
                projected: object
                    .get("id")
                    .and_then(Value::as_str)
                    .and_then(|value| Uuid::parse_str(value).ok()),
                unprojected: false,
            },
            other => AttachmentTarget {
                projected: other
                    .as_str()
                    .filter(|value| !value.is_empty())
                    .and_then(|value| Uuid::parse_str(value).ok()),
                unprojected: false,
            },
        };
    }

    AttachmentTarget {
        projected: None,
        unprojected: true,
    }
}

fn scan_record(record: &Value, object_name: &str, scan: &mut VisibilityScan) {
    match object_name {
        MESSAGE_OBJECT => match record_id(record) {
            Some(id) if holds_any_field(record, &RESTRICTED_MESSAGE_FIELDS) => {
                scan.message_ids.insert(id);
            }
            _ => {}
        },
        ATTACHMENT_OBJECT => {
            if !holds_any_field(record, &RESTRICTED_ATTACHMENT_FIELDS) {
                return;
            }
            let Some(id) = record_id(record) else {
                return;
            };

            if !scan.attachment_targets.iter().any(|(seen, _)| *seen == id) {
                scan.attachment_targets
                    .push((id, attachment_target_in_record(record)));
            }
        }
        CALENDAR_EVENT_OBJECT => match record_id(record) {
            Some(id) if holds_any_field(record, &RESTRICTED_CALENDAR_EVENT_FIELDS) => {
                scan.calendar_event_ids.insert(id);
            }
            _ => {}
        },
        _ => {}
    }
}

fn restricted_sentinel() -> Value {
    Value::String(FIELD_RESTRICTED_ADDITIONAL_PERMISSIONS_REQUIRED.to_string())
}

fn redact_message(record: &mut Value, visibility: &MessageVisibility) {
    if !holds_any_field(record, &RESTRICTED_MESSAGE_FIELDS) {
        return;
    }
    let id = record_id(record);

    if id.is_some_and(|id| visibility.fully_visible.contains(&id)) {
        return;
    }
    let subject_visible = id.is_some_and(|id| visibility.subject_visible.contains(&id));
    let Some(entries) = record.as_object_mut() else {
        return;
    };
    let sentinel = restricted_sentinel();

    if entries.contains_key("text") {
        entries.insert(String::from("text"), sentinel.clone());
    }

    if entries.contains_key("attachments") {
        entries.insert(String::from("attachments"), Value::Array(Vec::new()));
    }

    if !subject_visible && entries.contains_key("subject") {
        entries.insert(String::from("subject"), sentinel);
    }
}

fn redact_attachment(record: &mut Value, restricted_ids: &HashSet<Uuid>) {
    if !holds_any_field(record, &RESTRICTED_ATTACHMENT_FIELDS) {
        return;
    }

    if !record_id(record).is_some_and(|id| restricted_ids.contains(&id)) {
        return;
    }
    let Some(entries) = record.as_object_mut() else {
        return;
    };
    let sentinel = restricted_sentinel();

    for field_name in ["name", "fullPath"] {
        if entries.contains_key(field_name) {
            entries.insert(field_name.to_string(), sentinel.clone());
        }
    }

    if entries.contains_key("file") {
        entries.insert(String::from("file"), Value::Array(Vec::new()));
    }
}

fn redact_calendar_event(record: &mut Value, restricted_ids: &HashSet<Uuid>) {
    if !holds_any_field(record, &RESTRICTED_CALENDAR_EVENT_FIELDS) {
        return;
    }

    if !record_id(record).is_some_and(|id| restricted_ids.contains(&id)) {
        return;
    }
    let Some(entries) = record.as_object_mut() else {
        return;
    };
    let sentinel = restricted_sentinel();

    for field_name in RESTRICTED_CALENDAR_EVENT_FIELDS {
        if entries.contains_key(field_name) {
            entries.insert(field_name.to_string(), sentinel.clone());
        }
    }
}

fn redact_record(record: &mut Value, object_name: &str, decisions: &VisibilityDecisions) {
    match object_name {
        MESSAGE_OBJECT => redact_message(record, &decisions.message_visibility),
        ATTACHMENT_OBJECT => redact_attachment(record, &decisions.restricted_attachments),
        CALENDAR_EVENT_OBJECT => {
            redact_calendar_event(record, &decisions.restricted_calendar_events)
        }
        _ => {}
    }
}

fn uuid_cell(row: &Map<String, Value>, key: &str) -> Option<Uuid> {
    Uuid::parse_str(row.get(key)?.as_str()?).ok()
}

fn string_cell(row: &Map<String, Value>, key: &str) -> Option<String> {
    row.get(key)
        .and_then(Value::as_str)
        .map(|value| value.to_string())
}

fn put_ids_parameter(parameters: &mut Map<String, Value>, name: &str, ids: &[Uuid]) {
    parameters.insert(
        name.to_string(),
        Value::Array(ids.iter().map(|id| Value::String(id.to_string())).collect()),
    );
}

fn unique_sorted(ids: impl IntoIterator<Item = Uuid>) -> Vec<Uuid> {
    let mut ids = ids.into_iter().collect::<Vec<_>>();
    ids.sort();
    ids.dedup();

    ids
}

async fn select_rows(
    pool: &sqlx::PgPool,
    sql: &str,
    parameters: &Map<String, Value>,
) -> Result<Vec<Map<String, Value>>, RunnerError> {
    let compiled = compile_named_parameters(sql, parameters)
        .map_err(|error| RunnerError::new(error.code(), error.message()))?;

    execute_json_rows(pool, &compiled.text, &compiled.values).await
}

fn qualified_table(schema: &RunnerSchema, object_name: &str) -> Option<String> {
    let model = schema.object(object_name).ok()?;
    let schema_name = quoted_identifier(&model.shape.schema_name).ok()?;
    let table_name = quoted_identifier(&model.shape.table_name).ok()?;

    Some(format!("{schema_name}.{table_name}"))
}

fn column_for_field(model: &ObjectModel, field_name: &str) -> String {
    model
        .shape
        .columns
        .iter()
        .find(|column| column.field_name == field_name || column.column_name == field_name)
        .map(|column| column.column_name.clone())
        .unwrap_or_else(|| field_name.to_string())
}

async fn fetch_associations(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    object_name: &str,
    record_field: &str,
    channel_field: &str,
    record_ids: &[Uuid],
) -> Result<Vec<(Uuid, Uuid)>, RunnerError> {
    if record_ids.is_empty() {
        return Ok(Vec::new());
    }
    let Ok(model) = schema.object(object_name) else {
        return Ok(Vec::new());
    };
    let Some(table) = qualified_table(schema, object_name) else {
        return Ok(Vec::new());
    };
    let record_column = quoted_identifier(&column_for_field(model, record_field))?;
    let channel_column = quoted_identifier(&column_for_field(model, channel_field))?;
    let deleted_filter = if model.shape.has_deleted_at_column {
        format!(" AND a.{} IS NULL", quoted_identifier("deletedAt")?)
    } else {
        String::new()
    };
    let sql = format!(
        "SELECT a.{record_column} AS \"recordId\", a.{channel_column} AS \"channelId\" \
         FROM {table} AS a WHERE a.{record_column} = ANY(ARRAY[:...recordIds]::uuid[]){deleted_filter}"
    );
    let mut parameters = Map::new();
    put_ids_parameter(&mut parameters, "recordIds", record_ids);
    let rows = select_rows(pool, &sql, &parameters).await?;

    Ok(rows
        .iter()
        .filter_map(|row| Some((uuid_cell(row, "recordId")?, uuid_cell(row, "channelId")?)))
        .collect())
}

async fn fetch_channels(
    pool: &sqlx::PgPool,
    table: &str,
    workspace_id: Uuid,
    channel_ids: &[Uuid],
) -> Result<Vec<ChannelRow>, RunnerError> {
    if channel_ids.is_empty() {
        return Ok(Vec::new());
    }
    let channel_table = quoted_identifier(table)?;
    let account_table = quoted_identifier(CONNECTED_ACCOUNT_TABLE)?;
    let sql = format!(
        "SELECT ch.id AS \"channelId\", ch.visibility::text AS \"visibility\", \
         ca.\"userWorkspaceId\" AS \"userWorkspaceId\", ca.\"applicationId\" AS \"applicationId\" \
         FROM core.{channel_table} AS ch \
         LEFT JOIN core.{account_table} AS ca ON ca.id = ch.\"connectedAccountId\" \
         WHERE ch.\"workspaceId\" = :workspaceId AND ch.id = ANY(ARRAY[:...channelIds]::uuid[])"
    );
    let mut parameters = Map::new();
    parameters.insert(
        String::from("workspaceId"),
        Value::String(workspace_id.to_string()),
    );
    put_ids_parameter(&mut parameters, "channelIds", channel_ids);
    let rows = select_rows(pool, &sql, &parameters).await?;

    Ok(rows
        .iter()
        .filter_map(|row| {
            Some(ChannelRow {
                id: uuid_cell(row, "channelId")?,
                visibility: string_cell(row, "visibility").unwrap_or_default(),
                owner: ChannelOwner {
                    user_workspace_id: uuid_cell(row, "userWorkspaceId"),
                    application_id: uuid_cell(row, "applicationId"),
                },
            })
        })
        .collect())
}

async fn fetch_message_visibility(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    workspace_id: Uuid,
    message_ids: &[Uuid],
    viewer: ChannelViewer,
) -> Result<MessageVisibility, RunnerError> {
    let message_ids = unique_sorted(message_ids.iter().copied());

    if message_ids.is_empty() {
        return Ok(MessageVisibility::default());
    }
    let associations = fetch_associations(
        pool,
        schema,
        MESSAGE_CHANNEL_ASSOCIATION_OBJECT,
        "message",
        "messageChannel",
        &message_ids,
    )
    .await?;
    let channel_ids = unique_sorted(associations.iter().map(|(_, channel_id)| *channel_id));
    let channels = fetch_channels(pool, MESSAGE_CHANNEL_TABLE, workspace_id, &channel_ids).await?;

    Ok(resolve_message_visibility(&associations, &channels, viewer))
}

async fn fetch_visible_calendar_event_ids(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    workspace_id: Uuid,
    event_ids: &[Uuid],
    viewer: ChannelViewer,
) -> Result<HashSet<Uuid>, RunnerError> {
    let event_ids = unique_sorted(event_ids.iter().copied());

    if event_ids.is_empty() {
        return Ok(HashSet::new());
    }
    let associations = fetch_associations(
        pool,
        schema,
        CALENDAR_CHANNEL_ASSOCIATION_OBJECT,
        "calendarEvent",
        "calendarChannel",
        &event_ids,
    )
    .await?;
    let channel_ids = unique_sorted(associations.iter().map(|(_, channel_id)| *channel_id));
    let channels = fetch_channels(pool, CALENDAR_CHANNEL_TABLE, workspace_id, &channel_ids).await?;

    Ok(resolve_visible_calendar_event_ids(
        &associations,
        &channels,
        viewer,
    ))
}

async fn fetch_attachment_targets(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    attachment_ids: &[Uuid],
) -> Result<HashMap<Uuid, Uuid>, RunnerError> {
    if attachment_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let Some(table) = qualified_table(schema, ATTACHMENT_OBJECT) else {
        return Ok(HashMap::new());
    };
    let Some(target_column) = schema
        .object(ATTACHMENT_OBJECT)
        .ok()
        .and_then(|model| model.shape.relation(ATTACHMENT_TARGET_FIELD))
        .and_then(|relation| {
            relation
                .join_column_name
                .clone()
                .or_else(|| format!("{ATTACHMENT_TARGET_FIELD}Id").into())
        })
    else {
        return Ok(HashMap::new());
    };
    let target_column = quoted_identifier(&target_column)?;
    let sql = format!(
        "SELECT a.id AS \"attachmentId\", a.{target_column} AS \"targetMessageId\" \
         FROM {table} AS a WHERE a.id = ANY(ARRAY[:...attachmentIds]::uuid[]) \
         AND a.{target_column} IS NOT NULL"
    );
    let mut parameters = Map::new();
    put_ids_parameter(&mut parameters, "attachmentIds", attachment_ids);
    let rows = select_rows(pool, &sql, &parameters).await?;

    Ok(rows
        .iter()
        .filter_map(|row| {
            Some((
                uuid_cell(row, "attachmentId")?,
                uuid_cell(row, "targetMessageId")?,
            ))
        })
        .collect())
}

fn attachment_target_metadata_exists(schema: &RunnerSchema) -> bool {
    schema
        .object(ATTACHMENT_OBJECT)
        .ok()
        .and_then(|model| model.fields_by_name.get(ATTACHMENT_TARGET_FIELD).cloned())
        .is_some_and(|field| {
            field.field_type == FieldMetadataType::Relation
                || field.field_type == FieldMetadataType::MorphRelation
        })
}

pub fn channel_viewer_for_schema(schema: &RunnerSchema) -> ChannelViewer {
    ChannelViewer::from_credentials(schema.user_workspace_id(), None)
}

pub async fn apply_record_channel_visibility(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    records: &mut [Value],
) -> Result<(), RunnerError> {
    if records.is_empty() {
        return Ok(());
    }
    let Ok(model) = schema.object(object_name_singular) else {
        return Ok(());
    };
    let workspace_id = model.object.workspace_id;
    let viewer = channel_viewer_for_schema(schema);
    let mut scan = VisibilityScan::default();

    for record in records.iter_mut() {
        scan_record(record, object_name_singular, &mut scan);
        visit_nested_records(
            record,
            object_name_singular,
            schema,
            &mut |child, child_name| scan_record(child, child_name, &mut scan),
        );
    }

    if scan.is_empty() {
        return Ok(());
    }
    let track_attachments = attachment_target_metadata_exists(schema);
    let unresolved = scan
        .attachment_targets
        .iter()
        .filter(|(_, target)| target.unprojected)
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
    let stored_targets = fetch_attachment_targets(pool, schema, &unresolved).await?;
    let mut message_ids = scan.message_ids.iter().copied().collect::<Vec<_>>();
    let mut attachment_targets = Vec::new();

    for (attachment_id, target) in &scan.attachment_targets {
        if !track_attachments {
            continue;
        }
        let resolved = if target.unprojected {
            stored_targets.get(attachment_id).copied()
        } else {
            target.projected
        };

        if let Some(message_id) = resolved {
            message_ids.push(message_id);
        }
        attachment_targets.push((*attachment_id, resolved));
    }
    let message_visibility =
        fetch_message_visibility(pool, schema, workspace_id, &message_ids, viewer).await?;
    let visible_calendar_events = fetch_visible_calendar_event_ids(
        pool,
        schema,
        workspace_id,
        &scan.calendar_event_ids.iter().copied().collect::<Vec<_>>(),
        viewer,
    )
    .await?;
    let restricted_calendar_events = scan
        .calendar_event_ids
        .iter()
        .copied()
        .filter(|event_id| !visible_calendar_events.contains(event_id))
        .collect::<HashSet<_>>();
    let decisions = VisibilityDecisions {
        restricted_attachments: restricted_attachment_ids(
            &attachment_targets,
            &message_visibility.fully_visible,
        ),
        restricted_calendar_events,
        message_visibility,
    };

    for record in records.iter_mut() {
        redact_record(record, object_name_singular, &decisions);
        visit_nested_records(
            record,
            object_name_singular,
            schema,
            &mut |child, child_name| redact_record(child, child_name, &decisions),
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use uuid::Uuid;

    use super::*;
    use crate::metadata::flat::FlatEntityMaps;
    use crate::metadata::flat::FlatFieldMetadata;
    use crate::metadata::flat::FlatObjectMetadata;

    fn owner_channel(user_workspace_id: Uuid) -> ChannelOwner {
        ChannelOwner {
            user_workspace_id: Some(user_workspace_id),
            application_id: None,
        }
    }

    const ATTACHMENT_ID: &str = "6b0d1f2e-3a4b-4c5d-9e8f-0a1b2c3d4e5f";

    fn channel(visibility: &str, owner: ChannelOwner) -> ChannelRow {
        ChannelRow {
            id: Uuid::new_v4(),
            visibility: visibility.to_string(),
            owner,
        }
    }

    fn outsider() -> ChannelViewer {
        ChannelViewer::User {
            user_workspace_id: Uuid::new_v4(),
        }
    }

    fn body_message(id: Uuid) -> Value {
        json!({
            "id": id.to_string(),
            "subject": "Quarterly numbers",
            "text": "The attached deck has the quarterly numbers.",
            "attachments": [{ "id": ATTACHMENT_ID }],
        })
    }

    fn calendar_event(id: Uuid) -> Value {
        json!({
            "id": id.to_string(),
            "title": "Board sync",
            "description": "Vote on the acquisition.",
            "startsAt": "2026-01-01T09:00:00.000Z",
        })
    }

    #[test]
    fn message_channel_share_everything_exposes_the_body_to_another_member() {
        let viewer = outsider();
        let channel = channel("SHARE_EVERYTHING", ChannelOwner::default());
        let message_id = Uuid::new_v4();
        let visibility =
            resolve_message_visibility(&[(message_id, channel.id)], &[channel], viewer);

        assert!(visibility.fully_visible.contains(&message_id));
        assert!(visibility.subject_visible.is_empty());
    }

    #[test]
    fn message_channel_subject_visibility_exposes_only_the_subject() {
        let viewer = outsider();
        let channel = channel("SUBJECT", ChannelOwner::default());
        let message_id = Uuid::new_v4();
        let visibility =
            resolve_message_visibility(&[(message_id, channel.id)], &[channel], viewer);

        assert!(visibility.fully_visible.is_empty());
        assert!(visibility.subject_visible.contains(&message_id));
    }

    #[test]
    fn message_channel_metadata_visibility_hides_the_subject_and_the_body() {
        let viewer = outsider();
        let channel = channel("METADATA", ChannelOwner::default());
        let message_id = Uuid::new_v4();
        let visibility =
            resolve_message_visibility(&[(message_id, channel.id)], &[channel], viewer);

        assert!(visibility.fully_visible.is_empty());
        assert!(visibility.subject_visible.is_empty());
    }

    #[test]
    fn message_channel_owner_keeps_the_body_under_metadata_visibility() {
        let owner = Uuid::new_v4();
        let viewer = ChannelViewer::User {
            user_workspace_id: owner,
        };
        let channel = channel("METADATA", owner_channel(owner));
        let message_id = Uuid::new_v4();
        let visibility =
            resolve_message_visibility(&[(message_id, channel.id)], &[channel], viewer);

        assert!(visibility.fully_visible.contains(&message_id));
    }

    #[test]
    fn message_channel_owner_of_a_subject_channel_is_not_downgraded() {
        let owner = Uuid::new_v4();
        let viewer = ChannelViewer::User {
            user_workspace_id: owner,
        };
        let channel = channel("SUBJECT", owner_channel(owner));
        let message_id = Uuid::new_v4();
        let visibility =
            resolve_message_visibility(&[(message_id, channel.id)], &[channel], viewer);

        assert!(visibility.fully_visible.contains(&message_id));
        assert!(visibility.subject_visible.is_empty());
    }

    #[test]
    fn another_member_is_not_the_message_channel_owner() {
        let viewer = outsider();
        let channel = channel("METADATA", owner_channel(Uuid::new_v4()));
        let message_id = Uuid::new_v4();
        let visibility =
            resolve_message_visibility(&[(message_id, channel.id)], &[channel], viewer);

        assert!(visibility.fully_visible.is_empty());
        assert!(visibility.subject_visible.is_empty());
    }

    #[test]
    fn application_viewer_sees_bodies_on_channels_it_connected() {
        let application_id = Uuid::new_v4();
        let viewer = ChannelViewer::Application { application_id };
        let channel = ChannelRow {
            id: Uuid::new_v4(),
            visibility: "METADATA".to_string(),
            owner: ChannelOwner {
                user_workspace_id: None,
                application_id: Some(application_id),
            },
        };
        let message_id = Uuid::new_v4();
        let visibility =
            resolve_message_visibility(&[(message_id, channel.id)], &[channel], viewer);

        assert!(visibility.fully_visible.contains(&message_id));
    }

    #[test]
    fn application_viewer_never_owns_a_channel_it_did_not_connect() {
        let viewer = ChannelViewer::Application {
            application_id: Uuid::new_v4(),
        };
        let channel = channel("METADATA", owner_channel(Uuid::new_v4()));
        let message_id = Uuid::new_v4();
        let visibility =
            resolve_message_visibility(&[(message_id, channel.id)], &[channel], viewer);

        assert!(visibility.fully_visible.is_empty());
    }

    #[test]
    fn unassociated_message_is_hidden_from_every_viewer() {
        let viewer = outsider();
        let visibility = resolve_message_visibility(&[], &[], viewer);

        assert!(visibility.fully_visible.is_empty());
        assert!(visibility.subject_visible.is_empty());
    }

    #[test]
    fn message_in_several_channels_keeps_the_widest_visibility() {
        let viewer = outsider();
        let hidden = channel("METADATA", ChannelOwner::default());
        let shared = channel("SHARE_EVERYTHING", ChannelOwner::default());
        let message_id = Uuid::new_v4();
        let visibility = resolve_message_visibility(
            &[(message_id, hidden.id), (message_id, shared.id)],
            &[hidden, shared],
            viewer,
        );

        assert!(visibility.fully_visible.contains(&message_id));
        assert!(visibility.subject_visible.is_empty());
    }

    #[test]
    fn message_channel_from_another_workspace_does_not_grant_visibility() {
        let owner = Uuid::new_v4();
        let viewer = ChannelViewer::User {
            user_workspace_id: owner,
        };
        let message_id = Uuid::new_v4();
        let association = (message_id, Uuid::new_v4());
        let visibility = resolve_message_visibility(&[association], &[], viewer);

        assert!(visibility.fully_visible.is_empty());
        assert!(visibility.subject_visible.is_empty());
    }

    #[test]
    fn unknown_message_channel_visibility_is_treated_as_metadata() {
        assert_eq!(
            MessageChannelVisibility::parse("NOT_A_VISIBILITY"),
            MessageChannelVisibility::Metadata
        );
        assert_eq!(
            MessageChannelVisibility::parse("SUBJECT"),
            MessageChannelVisibility::Subject
        );
        assert_eq!(
            MessageChannelVisibility::parse("SHARE_EVERYTHING"),
            MessageChannelVisibility::ShareEverything
        );
    }

    #[test]
    fn subject_visibility_string_never_unlocks_calendar_events() {
        let viewer = outsider();
        let subject_channel = channel("SUBJECT", ChannelOwner::default());
        let event_id = Uuid::new_v4();
        let visible = resolve_visible_calendar_event_ids(
            &[(event_id, subject_channel.id)],
            &[subject_channel],
            viewer,
        );

        assert!(visible.is_empty());
    }

    #[test]
    fn calendar_channel_share_everything_exposes_events_to_another_member() {
        let viewer = outsider();
        let shared = channel("SHARE_EVERYTHING", ChannelOwner::default());
        let event_id = Uuid::new_v4();
        let visible =
            resolve_visible_calendar_event_ids(&[(event_id, shared.id)], &[shared], viewer);

        assert!(visible.contains(&event_id));
    }

    #[test]
    fn calendar_channel_metadata_visibility_hides_events_from_another_member() {
        let viewer = outsider();
        let hidden = channel("METADATA", owner_channel(Uuid::new_v4()));
        let event_id = Uuid::new_v4();
        let visible =
            resolve_visible_calendar_event_ids(&[(event_id, hidden.id)], &[hidden], viewer);

        assert!(!visible.contains(&event_id));
    }

    #[test]
    fn calendar_channel_owner_keeps_their_own_events() {
        let owner = Uuid::new_v4();
        let viewer = ChannelViewer::User {
            user_workspace_id: owner,
        };
        let hidden = channel("METADATA", owner_channel(owner));
        let event_id = Uuid::new_v4();
        let visible =
            resolve_visible_calendar_event_ids(&[(event_id, hidden.id)], &[hidden], viewer);

        assert!(visible.contains(&event_id));
    }

    #[test]
    fn application_viewer_never_owns_a_calendar_channel() {
        let application_id = Uuid::new_v4();
        let viewer = ChannelViewer::Application { application_id };
        let channel = ChannelRow {
            id: Uuid::new_v4(),
            visibility: "METADATA".to_string(),
            owner: ChannelOwner {
                user_workspace_id: None,
                application_id: Some(application_id),
            },
        };
        let event_id = Uuid::new_v4();
        let visible =
            resolve_visible_calendar_event_ids(&[(event_id, channel.id)], &[channel], viewer);

        assert!(visible.is_empty());
    }

    #[test]
    fn unassociated_calendar_event_is_hidden() {
        let viewer = outsider();
        let event_id = Uuid::new_v4();
        let visible = resolve_visible_calendar_event_ids(&[], &[], viewer);

        assert!(!visible.contains(&event_id));
    }

    #[test]
    fn attachment_is_restricted_when_its_message_is_not_fully_visible() {
        let message_id = Uuid::new_v4();
        let attachment_id = Uuid::new_v4();
        let restricted =
            restricted_attachment_ids(&[(attachment_id, Some(message_id))], &HashSet::new());

        assert!(restricted.contains(&attachment_id));
    }

    #[test]
    fn attachment_of_a_visible_message_keeps_its_name_and_file() {
        let message_id = Uuid::new_v4();
        let attachment_id = Uuid::new_v4();
        let mut visible = HashSet::new();
        visible.insert(message_id);
        let restricted = restricted_attachment_ids(&[(attachment_id, Some(message_id))], &visible);

        assert!(!restricted.contains(&attachment_id));
    }

    #[test]
    fn subject_visibility_alone_does_not_keep_the_attachment_visible() {
        let message_id = Uuid::new_v4();
        let attachment_id = Uuid::new_v4();
        let channel = channel("SUBJECT", ChannelOwner::default());
        let visibility =
            resolve_message_visibility(&[(message_id, channel.id)], &[channel], outsider());
        let restricted = restricted_attachment_ids(
            &[(attachment_id, Some(message_id))],
            &visibility.fully_visible,
        );

        assert!(restricted.contains(&attachment_id));
    }

    #[test]
    fn attachment_without_a_target_message_stays_readable() {
        let attachment_id = Uuid::new_v4();
        let mut visible_message = HashSet::new();
        visible_message.insert(Uuid::new_v4());
        let restricted = restricted_attachment_ids(&[(attachment_id, None)], &visible_message);

        assert!(restricted.is_empty());
    }

    #[test]
    fn channel_viewer_without_credentials_claims_no_ownership() {
        assert_eq!(
            ChannelViewer::from_credentials(None, None),
            ChannelViewer::NoOwnership
        );

        let user_workspace_id = Uuid::new_v4();
        assert_eq!(
            ChannelViewer::from_credentials(Some(user_workspace_id), None),
            ChannelViewer::User { user_workspace_id }
        );

        let application_id = Uuid::new_v4();
        assert_eq!(
            ChannelViewer::from_credentials(None, Some(application_id)),
            ChannelViewer::Application { application_id }
        );
        assert_eq!(
            ChannelViewer::from_credentials(Some(user_workspace_id), Some(application_id)),
            ChannelViewer::User { user_workspace_id }
        );
    }

    #[test]
    fn metadata_visibility_message_body_is_replaced_by_the_restriction_sentinel() {
        let message_id = Uuid::new_v4();
        let mut record = body_message(message_id);
        redact_message(&mut record, &MessageVisibility::default());

        assert_eq!(
            record["subject"],
            json!(FIELD_RESTRICTED_ADDITIONAL_PERMISSIONS_REQUIRED)
        );
        assert_eq!(
            record["text"],
            json!(FIELD_RESTRICTED_ADDITIONAL_PERMISSIONS_REQUIRED)
        );
        assert_eq!(record["attachments"], json!([]));
    }

    #[test]
    fn subject_visibility_message_keeps_the_subject_and_drops_the_body() {
        let message_id = Uuid::new_v4();
        let mut fully = MessageVisibility::default();
        fully.subject_visible.insert(message_id);
        let mut record = body_message(message_id);
        redact_message(&mut record, &fully);

        assert_eq!(record["subject"], json!("Quarterly numbers"));
        assert_eq!(
            record["text"],
            json!(FIELD_RESTRICTED_ADDITIONAL_PERMISSIONS_REQUIRED)
        );
        assert_eq!(record["attachments"], json!([]));
    }

    #[test]
    fn fully_visible_message_is_left_untouched_for_the_owner() {
        let message_id = Uuid::new_v4();
        let mut fully = MessageVisibility::default();
        fully.fully_visible.insert(message_id);
        let mut record = body_message(message_id);
        redact_message(&mut record, &fully);

        assert_eq!(record, body_message(message_id));
    }

    #[test]
    fn redaction_only_touches_the_fields_the_viewer_selected() {
        let message_id = Uuid::new_v4();
        let mut record = json!({ "id": message_id.to_string(), "text": "secret body" });
        redact_message(&mut record, &MessageVisibility::default());

        assert_eq!(
            record["text"],
            json!(FIELD_RESTRICTED_ADDITIONAL_PERMISSIONS_REQUIRED)
        );
        assert!(record.get("subject").is_none());
        assert!(record.get("attachments").is_none());
    }

    #[test]
    fn a_message_without_a_persistable_id_cannot_claim_visibility() {
        let mut record = json!({ "subject": "leaked", "text": "leaked body" });
        redact_message(&mut record, &MessageVisibility::default());

        assert_eq!(
            record["text"],
            json!(FIELD_RESTRICTED_ADDITIONAL_PERMISSIONS_REQUIRED)
        );
        assert_eq!(
            record["subject"],
            json!(FIELD_RESTRICTED_ADDITIONAL_PERMISSIONS_REQUIRED)
        );
    }

    #[test]
    fn restricted_calendar_event_loses_title_and_description_only() {
        let event_id = Uuid::new_v4();
        let mut record = calendar_event(event_id);
        let mut restricted = HashSet::new();
        restricted.insert(event_id);
        redact_calendar_event(&mut record, &restricted);

        assert_eq!(
            record["title"],
            json!(FIELD_RESTRICTED_ADDITIONAL_PERMISSIONS_REQUIRED)
        );
        assert_eq!(
            record["description"],
            json!(FIELD_RESTRICTED_ADDITIONAL_PERMISSIONS_REQUIRED)
        );
        assert_eq!(record["startsAt"], json!("2026-01-01T09:00:00.000Z"));
    }

    #[test]
    fn visible_calendar_event_keeps_its_title_for_the_owner() {
        let event_id = Uuid::new_v4();
        let mut record = calendar_event(event_id);
        redact_calendar_event(&mut record, &HashSet::new());

        assert_eq!(record, calendar_event(event_id));
    }

    #[test]
    fn restricted_attachment_empties_the_file_list() {
        let attachment_id = Uuid::new_v4();
        let mut record = json!({
            "id": attachment_id.to_string(),
            "name": "deck.pdf",
            "fullPath": "core-attachments/deck.pdf",
            "file": [{ "version": 1 }],
        });
        let mut restricted = HashSet::new();
        restricted.insert(attachment_id);
        redact_attachment(&mut record, &restricted);

        assert_eq!(
            record["name"],
            json!(FIELD_RESTRICTED_ADDITIONAL_PERMISSIONS_REQUIRED)
        );
        assert_eq!(
            record["fullPath"],
            json!(FIELD_RESTRICTED_ADDITIONAL_PERMISSIONS_REQUIRED)
        );
        assert_eq!(record["file"], json!([]));
    }

    #[test]
    fn attachment_target_is_read_from_the_projected_join_column() {
        let message_id = Uuid::new_v4();
        let projected =
            attachment_target_in_record(&json!({ "targetMessageId": message_id.to_string() }));

        assert!(!projected.unprojected);
        assert_eq!(projected.projected, Some(message_id));

        let null_target = attachment_target_in_record(&json!({ "targetMessageId": Value::Null }));

        assert!(!null_target.unprojected);
        assert_eq!(null_target.projected, None);

        let loaded_relation = attachment_target_in_record(
            &json!({ "targetMessage": { "id": message_id.to_string() } }),
        );

        assert!(!loaded_relation.unprojected);
        assert_eq!(loaded_relation.projected, Some(message_id));

        let absent = attachment_target_in_record(&json!({ "name": "deck.pdf" }));

        assert!(absent.unprojected);
    }

    fn plain_field(object_id: Uuid, name: &str) -> FlatFieldMetadata {
        FlatFieldMetadata {
            id: Uuid::new_v4(),
            object_metadata_id: object_id,
            field_type: if name == "id" {
                FieldMetadataType::Uuid
            } else {
                FieldMetadataType::Text
            },
            name: name.to_string(),
            label: name.to_string(),
            ..Default::default()
        }
    }

    fn relation_field(
        object_id: Uuid,
        name: &str,
        target_object_id: Uuid,
        relation_type: &str,
    ) -> FlatFieldMetadata {
        FlatFieldMetadata {
            id: Uuid::new_v4(),
            object_metadata_id: object_id,
            field_type: FieldMetadataType::Relation,
            name: name.to_string(),
            label: name.to_string(),
            relation_target_object_metadata_id: Some(target_object_id),
            settings: Some(json!({ "relationType": relation_type })),
            ..Default::default()
        }
    }

    fn thread_object(
        workspace_id: Uuid,
        id: Uuid,
        name: &str,
        fields: &[FlatFieldMetadata],
    ) -> FlatObjectMetadata {
        FlatObjectMetadata {
            id,
            workspace_id,
            name_singular: name.to_string(),
            name_plural: format!("{name}s"),
            label_singular: name.to_string(),
            label_plural: format!("{name}s"),
            target_table_name: Some(name.to_string()),
            field_ids: fields.iter().map(|field| field.id.to_string()).collect(),
            ..Default::default()
        }
    }

    fn thread_schema() -> RunnerSchema {
        let workspace_id = Uuid::new_v4();
        let thread_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();
        let mut message_fields = vec![
            plain_field(message_id, "id"),
            plain_field(message_id, "subject"),
            plain_field(message_id, "text"),
        ];
        message_fields.push(relation_field(
            message_id,
            "messageThread",
            thread_id,
            "MANY_TO_ONE",
        ));
        let mut thread_fields = vec![plain_field(thread_id, "id")];
        thread_fields.push(relation_field(thread_id, "messages", message_id, "ONE_TO_MANY"));
        let mut fields = FlatEntityMaps::default();

        for field in message_fields.iter().chain(thread_fields.iter()) {
            fields.add(
                field.id.to_string(),
                field.clone(),
                field.id.to_string(),
                workspace_id.to_string(),
            );
        }
        let objects = vec![
            thread_object(workspace_id, thread_id, "messageThread", &thread_fields),
            thread_object(workspace_id, message_id, "message", &message_fields),
        ];

        RunnerSchema::build(
            &workspace_id,
            &objects,
            &fields,
            &FlatEntityMaps::default(),
        )
        .expect("thread schema")
    }

    fn thread_with_messages(message_id: Uuid) -> Value {
        json!({
            "id": Uuid::new_v4().to_string(),
            "messages": [{
                "id": message_id.to_string(),
                "subject": "Quarterly numbers",
                "text": "The attached deck has the quarterly numbers.",
            }],
        })
    }

    #[test]
    fn messages_nested_under_a_thread_are_scanned() {
        let schema = thread_schema();
        let message_id = Uuid::new_v4();
        let mut thread = thread_with_messages(message_id);
        let mut scan = VisibilityScan::default();
        scan_record(&thread, "messageThread", &mut scan);
        visit_nested_records(
            &mut thread,
            "messageThread",
            &schema,
            &mut |child, child_name| scan_record(child, child_name, &mut scan),
        );

        assert!(scan.message_ids.contains(&message_id));
    }

    #[test]
    fn a_thread_read_hides_the_nested_message_body_from_a_non_owner() {
        let schema = thread_schema();
        let message_id = Uuid::new_v4();
        let mut thread = thread_with_messages(message_id);
        let decisions = VisibilityDecisions::default();
        redact_record(&mut thread, "messageThread", &decisions);
        visit_nested_records(
            &mut thread,
            "messageThread",
            &schema,
            &mut |child, child_name| redact_record(child, child_name, &decisions),
        );

        assert_eq!(
            thread["messages"][0]["text"],
            json!(FIELD_RESTRICTED_ADDITIONAL_PERMISSIONS_REQUIRED)
        );
        assert_eq!(
            thread["messages"][0]["subject"],
            json!(FIELD_RESTRICTED_ADDITIONAL_PERMISSIONS_REQUIRED)
        );
    }

    #[test]
    fn a_thread_read_keeps_the_nested_message_body_for_the_channel_owner() {
        let schema = thread_schema();
        let message_id = Uuid::new_v4();
        let mut thread = thread_with_messages(message_id);
        let mut visibility = MessageVisibility::default();
        visibility.fully_visible.insert(message_id);
        let decisions = VisibilityDecisions {
            message_visibility: visibility,
            ..Default::default()
        };
        redact_record(&mut thread, "messageThread", &decisions);
        visit_nested_records(
            &mut thread,
            "messageThread",
            &schema,
            &mut |child, child_name| redact_record(child, child_name, &decisions),
        );

        assert_eq!(
            thread["messages"][0]["text"],
            json!("The attached deck has the quarterly numbers.")
        );
    }

    #[test]
    fn reading_an_unknown_object_leaves_records_untouched() {
        let schema = thread_schema();
        let mut records = vec![json!({ "id": Uuid::new_v4().to_string(), "title": "keep me" })];
        let before = records.clone();

        assert!(schema.object("task").is_err());
        visit_nested_records(&mut records[0], "task", &schema, &mut |_, _| {});

        assert_eq!(records, before);
    }

    #[test]
    fn a_schema_without_a_user_workspace_claims_no_channel_ownership() {
        let schema = thread_schema();
        assert_eq!(channel_viewer_for_schema(&schema), ChannelViewer::NoOwnership);

        let user_workspace_id = Uuid::new_v4();
        assert_eq!(
            channel_viewer_for_schema(&schema.with_user_workspace_id(Some(user_workspace_id))),
            ChannelViewer::User {
                user_workspace_id
            }
        );
    }
}
