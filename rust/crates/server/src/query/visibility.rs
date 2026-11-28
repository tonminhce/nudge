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
use crate::auth::jwt_service::TokenType;
use crate::auth::middleware::AuthContext;
use crate::metadata::flat::FieldMetadataType;
use crate::metadata::flat::FlatFieldMetadata;
use crate::metadata::flat::RelationType;
use crate::permissions::ObjectRecordsOperationType;

pub const FIELD_RESTRICTED_ADDITIONAL_PERMISSIONS_REQUIRED: &str =
    "FIELD_RESTRICTED_ADDITIONAL_PERMISSIONS_REQUIRED";

const MESSAGE_OBJECT: &str = "message";
const ATTACHMENT_OBJECT: &str = "attachment";
const CALENDAR_EVENT_OBJECT: &str = "calendarEvent";
const TIMELINE_ACTIVITY_OBJECT: &str = "timelineActivity";
const MESSAGE_CHANNEL_ASSOCIATION_OBJECT: &str = "messageChannelMessageAssociation";
const CALENDAR_CHANNEL_ASSOCIATION_OBJECT: &str = "calendarChannelEventAssociation";

const MESSAGE_CHANNEL_TABLE: &str = "messageChannel";
const CALENDAR_CHANNEL_TABLE: &str = "calendarChannel";
const CONNECTED_ACCOUNT_TABLE: &str = "connectedAccount";
const USER_WORKSPACE_TABLE: &str = "userWorkspace";

const RESTRICTED_MESSAGE_FIELDS: [&str; 3] = ["subject", "text", "attachments"];
const RESTRICTED_ATTACHMENT_FIELDS: [&str; 3] = ["name", "fullPath", "file"];
const RESTRICTED_CALENDAR_EVENT_FIELDS: [&str; 2] = ["title", "description"];
const RESTRICTED_TIMELINE_ACTIVITY_FIELDS: [&str; 1] = ["properties"];

const ATTACHMENT_TARGET_FIELD: &str = "targetMessage";
const LINKED_OBJECT_METADATA_FIELD: &str = "linkedObjectMetadataId";
const TIMELINE_TARGET_PREFIX: &str = "target";
const EVENT_PROPERTY_CONTAINERS: [&str; 3] = ["before", "after", "diff"];
const UPDATED_FIELDS_PROPERTY: &str = "updatedFields";

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChannelOwner {
    pub user_workspace_id: Option<Uuid>,
    pub application_id: Option<Uuid>,
}

impl ChannelOwner {
    pub fn owns_messages(&self, viewer: ChannelViewer) -> bool {
        match viewer {
            ChannelViewer::User {
                user_workspace_id,
            } => self.user_workspace_id == Some(user_workspace_id),
            ChannelViewer::Application { application_id } => {
                self.application_id == Some(application_id)
            }
            ChannelViewer::NoOwnership => false,
        }
    }

    pub fn owns_calendar_events(&self, viewer: ChannelViewer) -> bool {
        match viewer {
            ChannelViewer::User {
                user_workspace_id,
            } => self.user_workspace_id == Some(user_workspace_id),
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

pub fn filter_field_names_in_event_properties(
    properties: &Map<String, Value>,
    readable_field_names: &HashSet<String>,
) -> Map<String, Value> {
    let mut filtered = Map::new();

    for container in EVENT_PROPERTY_CONTAINERS {
        let Some(value) = properties.get(container) else {
            continue;
        };

        if value.is_null() {
            continue;
        }

        match value.as_object() {
            Some(entries) => {
                let kept = entries
                    .iter()
                    .filter(|(field_name, _)| readable_field_names.contains(field_name.as_str()))
                    .map(|(field_name, value)| (field_name.clone(), value.clone()))
                    .collect::<Map<_, _>>();
                filtered.insert(container.to_string(), Value::Object(kept));
            }
            None => {
                filtered.insert(container.to_string(), Value::Null);
            }
        }
    }

    if let Some(Value::Array(updated_fields)) = properties.get(UPDATED_FIELDS_PROPERTY) {
        let kept = updated_fields
            .iter()
            .filter(|entry| {
                entry
                    .as_str()
                    .is_some_and(|name| readable_field_names.contains(name))
            })
            .cloned()
            .collect::<Vec<_>>();
        filtered.insert(String::from(UPDATED_FIELDS_PROPERTY), Value::Array(kept));
    }

    filtered
}

pub fn omit_restricted_fields_from_timeline_activity(
    properties: &Value,
    readable_field_names: &HashSet<String>,
) -> Value {
    let Some(entries) = properties.as_object() else {
        return Value::Null;
    };
    let filtered = filter_field_names_in_event_properties(entries, readable_field_names);
    let mut redacted = entries.clone();

    for container in EVENT_PROPERTY_CONTAINERS {
        if !entries.contains_key(container) {
            continue;
        }

        match filtered.get(container) {
            Some(Value::Null) => {
                redacted.remove(container);
            }
            Some(value) => {
                redacted.insert(container.to_string(), value.clone());
            }
            None => {}
        }
    }

    if let Some(value) = filtered.get(UPDATED_FIELDS_PROPERTY) {
        redacted.insert(String::from(UPDATED_FIELDS_PROPERTY), value.clone());
    }

    Value::Object(redacted)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivityObject {
    pub object_metadata_id: Option<Uuid>,
    pub is_linked: bool,
}

#[derive(Debug, Clone)]
struct TimelineTarget {
    field_name: String,
    join_column: String,
    object_metadata_id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AttachmentTarget {
    Projected(Option<Uuid>),
    Unprojected,
}

#[derive(Debug, Default)]
struct VisibilityScan {
    message_ids: HashSet<Uuid>,
    calendar_event_ids: HashSet<Uuid>,
    attachment_targets: Vec<(Uuid, AttachmentTarget)>,
    activity_ids: Vec<Uuid>,
    activity_objects: HashMap<Uuid, ActivityObject>,
}

impl VisibilityScan {
    fn is_empty(&self) -> bool {
        self.message_ids.is_empty()
            && self.calendar_event_ids.is_empty()
            && self.attachment_targets.is_empty()
            && self.activity_ids.is_empty()
    }
}

#[derive(Debug, Default)]
struct VisibilityDecisions {
    message_visibility: MessageVisibility,
    restricted_attachments: HashSet<Uuid>,
    restricted_calendar_events: HashSet<Uuid>,
    readable_properties: HashMap<Uuid, HashSet<String>>,
}

fn quoted_identifier(identifier: &str) -> Result<String, RunnerError> {
    quote_identifier(identifier).map_err(RunnerError::from_repository)
}

fn record_id(record: &Value) -> Option<Uuid> {
    Uuid::parse_str(record.get("id")?.as_str()?).ok()
}

fn non_empty_uuid(record: &Value, field_name: &str) -> Option<Uuid> {
    record
        .get(field_name)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .and_then(|value| Uuid::parse_str(value).ok())
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

fn visit_child_records<F>(
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
            Value::Object(_) => visit_child_records(child, &child_name, schema, visit),
            Value::Array(items) => {
                for item in items.iter_mut() {
                    if item.is_object() {
                        visit_child_records(item, &child_name, schema, visit);
                    }
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
            Value::Null => AttachmentTarget::Projected(None),
            Value::Object(object) => AttachmentTarget::Projected(
                object
                    .get("id")
                    .and_then(Value::as_str)
                    .and_then(|value| Uuid::parse_str(value).ok()),
            ),
            other => AttachmentTarget::Projected(
                other
                    .as_str()
                    .filter(|value| !value.is_empty())
                    .and_then(|value| Uuid::parse_str(value).ok()),
            ),
        };
    }

    AttachmentTarget::Unprojected
}

pub fn resolve_activity_object(record: &Value, targets: &[TimelineTarget]) -> Option<ActivityObject> {
    let entries = record.as_object()?;
    let discriminator = format!("{LINKED_OBJECT_METADATA_FIELD}Id");

    if !entries.contains_key(LINKED_OBJECT_METADATA_FIELD) && !entries.contains_key(&discriminator)
    {
        return None;
    }
    let linked_object_metadata_id = non_empty_uuid(record, LINKED_OBJECT_METADATA_FIELD)
        .or_else(|| non_empty_uuid(record, &discriminator));

    if let Some(object_metadata_id) = linked_object_metadata_id {
        return Some(ActivityObject {
            object_metadata_id: Some(object_metadata_id),
            is_linked: true,
        });
    }

    for target in targets {
        let projected = [target.join_column.as_str(), target.field_name.as_str()]
            .into_iter()
            .any(|key| {
                entries
                    .get(key)
                    .and_then(Value::as_str)
                    .is_some_and(|value| !value.is_empty())
            });

        if projected {
            return Some(ActivityObject {
                object_metadata_id: Some(target.object_metadata_id),
                is_linked: false,
            });
        }
    }

    Some(ActivityObject {
        object_metadata_id: None,
        is_linked: false,
    })
}

fn scan_record(record: &mut Value, object_name: &str, targets: &[TimelineTarget], scan: &mut VisibilityScan) {
    match object_name {
        MESSAGE_OBJECT => {
            if holds_any_field(record, &RESTRICTED_MESSAGE_FIELDS) {
                if let Some(id) = record_id(record) {
                    scan.message_ids.insert(id);
                }
            }
        }
        ATTACHMENT_OBJECT => {
            if !holds_any_field(record, &RESTRICTED_ATTACHMENT_FIELDS) {
                return;
            }
            let Some(id) = record_id(record) else {
                return;
            };

            if !scan
                .attachment_targets
                .iter()
                .any(|(seen, _)| *seen == id)
            {
                scan.attachment_targets
                    .push((id, attachment_target_in_record(record)));
            }
        }
        CALENDAR_EVENT_OBJECT => {
            if holds_any_field(record, &RESTRICTED_CALENDAR_EVENT_FIELDS) {
                if let Some(id) = record_id(record) {
                    scan.calendar_event_ids.insert(id);
                }
            }
        }
        TIMELINE_ACTIVITY_OBJECT => {
            if !holds_any_field(record, &RESTRICTED_TIMELINE_ACTIVITY_FIELDS) {
                return;
            }
            let Some(id) = record_id(record) else {
                return;
            };

            if !scan.activity_ids.contains(&id) {
                scan.activity_ids.push(id);
            }

            if !scan.activity_objects.contains_key(&id) {
                if let Some(object) = resolve_activity_object(record, targets) {
                    scan.activity_objects.insert(id, object);
                }
            }
        }
        _ => {}
    }
}

fn timeline_targets(schema: &RunnerSchema) -> Vec<TimelineTarget> {
    let Ok(model) = schema.object(TIMELINE_ACTIVITY_OBJECT) else {
        return Vec::new();
    };
    let mut targets = model
        .shape
        .relation_shape_by_field_name
        .values()
        .filter(|relation| relation.field_name.starts_with(TIMELINE_TARGET_PREFIX))
        .filter(|relation| relation.relation_type == RelationType::ManyToOne)
        .filter_map(|relation| {
            let object_metadata_id = relation.target_object_metadata_id?;

            Some(TimelineTarget {
                field_name: relation.field_name.clone(),
                join_column: relation
                    .join_column_name
                    .clone()
                    .unwrap_or_else(|| format!("{}Id", relation.field_name)),
                object_metadata_id,
            })
        })
        .collect::<Vec<_>>();
    targets.sort_by(|left, right| left.join_column.cmp(&right.join_column));

    targets
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

fn object_read_denied(schema: &RunnerSchema, object_metadata_id: Uuid) -> bool {
    schema
        .records_permissions()
        .and_then(|permissions| permissions.object(object_metadata_id))
        .is_some_and(|object| {
            !object.operation_allowed(ObjectRecordsOperationType::Read)
                && !object.can_read_some_records()
        })
}

fn event_property_field_names(field: &FlatFieldMetadata, model: &ObjectModel) -> Vec<String> {
    if !matches!(
        field.field_type,
        FieldMetadataType::Relation | FieldMetadataType::MorphRelation
    ) {
        return vec![field.name.clone()];
    }
    let many_to_one = model
        .shape
        .relation(&field.name)
        .is_some_and(|relation| relation.relation_type == RelationType::ManyToOne);

    if many_to_one {
        return vec![field.name.clone(), format!("{}Id", field.name)];
    }

    vec![field.name.clone()]
}

pub fn readable_event_property_names(
    schema: &RunnerSchema,
    object: &ActivityObject,
) -> HashSet<String> {
    let Some(object_metadata_id) = object.object_metadata_id else {
        return HashSet::new();
    };
    let Ok(model) = schema.object_by_id(&object_metadata_id) else {
        return HashSet::new();
    };

    if object.is_linked && object_read_denied(schema, model.object.id) {
        return HashSet::new();
    }
    let mut readable = HashSet::new();

    for field in model.fields_by_name.values() {
        if schema.is_field_unreadable(model.object.id, field.id) {
            continue;
        }

        for name in event_property_field_names(field, model) {
            readable.insert(name);
        }
    }

    readable
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
    let sentinel = Value::String(FIELD_RESTRICTED_ADDITIONAL_PERMISSIONS_REQUIRED.to_string());

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
    let sentinel = Value::String(FIELD_RESTRICTED_ADDITIONAL_PERMISSIONS_REQUIRED.to_string());

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
    let sentinel = Value::String(FIELD_RESTRICTED_ADDITIONAL_PERMISSIONS_REQUIRED.to_string());

    for field_name in RESTRICTED_CALENDAR_EVENT_FIELDS {
        if entries.contains_key(field_name) {
            entries.insert(field_name.to_string(), sentinel.clone());
        }
    }
}

fn redact_timeline_activity(
    record: &mut Value,
    schema: &RunnerSchema,
    decisions: &VisibilityDecisions,
    targets: &[TimelineTarget],
) {
    if !holds_any_field(record, &RESTRICTED_TIMELINE_ACTIVITY_FIELDS) {
        return;
    }
    let Some(entries) = record.as_object_mut() else {
        return;
    };
    let Some(properties) = entries.get("properties").cloned() else {
        return;
    };
    let empty = HashSet::new();
    let resolved = record_id(record)
        .and_then(|id| decisions.readable_properties.get(&id))
        .cloned();
    let readable = match resolved {
        Some(readable) => readable,
        None => match resolve_activity_object(record, targets) {
            Some(object) => readable_event_property_names(schema, &object),
            None => empty.clone(),
        },
    };
    let redacted = omit_restricted_fields_from_timeline_activity(&properties, &readable);
    entries.insert(String::from("properties"), redacted);
}

fn apply_record_decisions(
    record: &mut Value,
    object_name: &str,
    schema: &RunnerSchema,
    decisions: &VisibilityDecisions,
    targets: &[TimelineTarget],
) {
    match object_name {
        MESSAGE_OBJECT => redact_message(record, &decisions.message_visibility),
        ATTACHMENT_OBJECT => redact_attachment(record, &decisions.restricted_attachments),
        CALENDAR_EVENT_OBJECT => {
            redact_calendar_event(record, &decisions.restricted_calendar_events)
        }
        TIMELINE_ACTIVITY_OBJECT => redact_timeline_activity(record, decisions, targets),
        _ => {}
    }

    visit_child_records(record, object_name, schema, &mut |child, child_name| {
        apply_record_decisions(child, child_name, schema, decisions, targets)
    });
}

fn uuid_cell(row: &Map<String, Value>, key: &str) -> Option<Uuid> {
    Uuid::parse_str(row.get(key)?.as_str()?).ok()
}

fn string_cell(row: &Map<String, Value>, key: &str) -> Option<String> {
    row.get(key)
        .and_then(Value::as_str)
        .map(|value| value.to_string())
}

fn ids_parameter(name: &str, ids: &[Uuid]) -> (String, Value) {
    (
        name.to_string(),
        Value::Array(ids.iter().map(|id| Value::String(id.to_string())).collect()),
    )
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
    let Some(table) = qualified_table(schema, object_name) else {
        return Ok(Vec::new());
    };
    let Ok(model) = schema.object(object_name) else {
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
    parameters.insert(ids_parameter("recordIds", record_ids));
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
    parameters.insert(ids_parameter("channelIds", channel_ids));
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
    let channels =
        fetch_channels(pool, MESSAGE_CHANNEL_TABLE, workspace_id, &channel_ids).await?;

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
        &associations, &channels, viewer,
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
    parameters.insert(ids_parameter("attachmentIds", attachment_ids));
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

async fn fetch_activity_objects(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    activity_ids: &[Uuid],
    targets: &[TimelineTarget],
) -> Result<HashMap<Uuid, ActivityObject>, RunnerError> {
    if activity_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let Some(table) = qualified_table(schema, TIMELINE_ACTIVITY_OBJECT) else {
        return Ok(HashMap::new());
    };
    let Ok(model) = schema.object(TIMELINE_ACTIVITY_OBJECT) else {
        return Ok(HashMap::new());
    };
    let mut projections = Vec::new();

    if model
        .shape
        .columns
        .iter()
        .any(|column| column.column_name == LINKED_OBJECT_METADATA_FIELD)
    {
        projections.push((
            quoted_identifier(LINKED_OBJECT_METADATA_FIELD)?,
            String::from(LINKED_OBJECT_METADATA_FIELD),
        ));
    }

    for target in targets {
        let column = column_for_field(model, &target.field_name);

        if !model.shape.columns.iter().any(|item| item.column_name == column) {
            continue;
        }
        projections.push((quoted_identifier(&column)?, target.field_name.clone()));
    }
    let selected = projections
        .iter()
        .map(|(column, alias)| format!("{column} AS {alias}"))
        .collect::<Vec<_>>()
        .join(", ");
    let selected = if selected.is_empty() {
        String::from("NULL AS \"unprojected\"")
    } else {
        selected
    };
    let sql = format!(
        "SELECT a.id AS \"activityId\", {selected} FROM {table} AS a \
         WHERE a.id = ANY(ARRAY[:...activityIds]::uuid[])"
    );
    let mut parameters = Map::new();
    parameters.insert(ids_parameter("activityIds", activity_ids));
    let rows = select_rows(pool, &sql, &parameters).await?;
    let mut objects = HashMap::new();

    for row in &rows {
        let Some(activity_id) = uuid_cell(row, "activityId") else {
            continue;
        };
        let linked_object_metadata_id = uuid_cell(row, LINKED_OBJECT_METADATA_FIELD);
        let object_metadata_id = match linked_object_metadata_id {
            Some(object_metadata_id) => Some(object_metadata_id),
            None => targets
                .iter()
                .find(|target| {
                    row.get(target.field_name.as_str())
                        .and_then(Value::as_str)
                        .is_some_and(|value| !value.is_empty())
                })
                .map(|target| target.object_metadata_id),
        };

        objects.insert(
            activity_id,
            ActivityObject {
                object_metadata_id,
                is_linked: linked_object_metadata_id.is_some(),
            },
        );
    }

    Ok(objects)
}

pub async fn channel_viewer_for_auth(
    pool: &sqlx::PgPool,
    auth: &AuthContext,
) -> ChannelViewer {
    if auth.token_type == TokenType::ApiKey {
        return ChannelViewer::NoOwnership;
    }

    if let Some(user_workspace_id) = auth.user_workspace_id {
        return ChannelViewer::User {
            user_workspace_id,
        };
    }
    let (Some(user_id), Some(workspace_id)) = (auth.user_id, auth.workspace_id) else {
        return ChannelViewer::NoOwnership;
    };
    let Ok(table) = quoted_identifier(USER_WORKSPACE_TABLE) else {
        return ChannelViewer::NoOwnership;
    };
    let sql = format!(
        "SELECT uw.id AS \"id\" FROM core.{table} AS uw \
         WHERE uw.\"userId\" = :userId::uuid AND uw.\"workspaceId\" = :workspaceId::uuid LIMIT 1"
    );
    let mut parameters = Map::new();
    parameters.insert(String::from("userId"), Value::String(user_id.to_string()));
    parameters.insert(
        String::from("workspaceId"),
        Value::String(workspace_id.to_string()),
    );
    let Ok(mut rows) = select_rows(pool, &sql, &parameters).await else {
        return ChannelViewer::NoOwnership;
    };

    rows.iter_mut()
        .next()
        .and_then(|row| uuid_cell(row, "id"))
        .map_or(ChannelViewer::NoOwnership, |user_workspace_id| {
            ChannelViewer::User {
                user_workspace_id,
            }
        })
}

pub async fn apply_record_visibility_restrictions(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    records: &mut [Value],
    viewer: ChannelViewer,
) -> Result<(), RunnerError> {
    if records.is_empty() {
        return Ok(());
    }
    let workspace_id = match schema.object(object_name_singular) {
        Ok(model) => model.object.workspace_id,
        Err(_) => return Ok(()),
    };
    let targets = timeline_targets(schema);
    let mut scan = VisibilityScan::default();

    for record in records.iter_mut() {
        scan_record(record, object_name_singular, &targets, &mut scan);
        visit_child_records(record, object_name_singular, schema, &mut |child, child_name| {
            scan_record(child, child_name, &targets, &mut scan)
        });
    }

    if scan.is_empty() {
        return Ok(());
    }
    let track_attachments = attachment_target_metadata_exists(schema);
    let unresolved = scan
        .attachment_targets
        .iter()
        .filter(|(_, target)| matches!(target, AttachmentTarget::Unprojected))
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
    let stored_targets = fetch_attachment_targets(pool, schema, &unresolved).await?;
    let mut message_ids = scan.message_ids.iter().copied().collect::<Vec<_>>();
    let mut attachment_targets = Vec::new();

    for (attachment_id, target) in &scan.attachment_targets {
        let resolved = match target {
            AttachmentTarget::Projected(target) => *target,
            AttachmentTarget::Unprojected => stored_targets.get(attachment_id).copied(),
        };

        if !track_attachments {
            continue;
        }

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
    let unresolved_activities = scan
        .activity_ids
        .iter()
        .filter(|activity_id| !scan.activity_objects.contains_key(activity_id))
        .copied()
        .collect::<Vec<_>>();
    let mut activity_objects = scan.activity_objects.clone();
    let fetched_objects =
        fetch_activity_objects(pool, schema, &unresolved_activities, &targets).await?;

    for (activity_id, object) in fetched_objects {
        activity_objects.insert(activity_id, object);
    }
    let mut readable_properties = HashMap::new();

    for activity_id in &scan.activity_ids {
        let object = activity_objects
            .get(activity_id)
            .copied()
            .unwrap_or(ActivityObject {
                object_metadata_id: None,
                is_linked: false,
            });
        readable_properties
            .insert(*activity_id, readable_event_property_names(schema, &object));
    }
    let decisions = VisibilityDecisions {
        message_visibility,
        restricted_attachments: restricted_attachment_ids(
            &attachment_targets,
            &message_visibility.fully_visible,
        ),
        restricted_calendar_events,
        readable_properties,
    };

    for record in records.iter_mut() {
        apply_record_decisions(record, object_name_singular, schema, &decisions, &targets);
    }

    Ok(())
}
