use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use serde_json::Map;
use serde_json::Value;

use crate::query::filter::record_matches_filter;
use crate::query::filter::SnapshotFilterSchema;
use crate::query::runner::actors::ActorContext;
use crate::query::runner::actors::ActorSource;
use crate::query::runner::QUERY_MAX_RECORDS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecordOperationSignature {
    pub object_name_singular: String,
    pub variables: RecordOperationVariables,
    #[serde(default)]
    pub fields: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecordOperationVariables {
    #[serde(default)]
    pub filter: Option<Value>,
    #[serde(default)]
    pub order_by: Option<Value>,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub cursor_filter: Option<RecordCursorFilter>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecordCursorFilter {
    pub cursor: String,
    pub cursor_direction: RecordCursorDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordCursorDirection {
    Before,
    After,
}

impl RecordOperationSignature {
    pub fn new(
        object_name_singular: impl Into<String>,
        variables: RecordOperationVariables,
        fields: Option<Map<String, Value>>,
    ) -> Self {
        Self {
            object_name_singular: object_name_singular.into(),
            variables,
            fields,
        }
    }

    pub fn parse(value: Value) -> Result<Self, String> {
        let signature = serde_json::from_value::<Self>(value)
            .map_err(|error| format!("Invalid operationSignature: {error}"))?;
        signature.validate()?;
        Ok(signature)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.object_name_singular.trim().is_empty() {
            return Err(String::from(
                "operationSignature.objectNameSingular is required",
            ));
        }
        if self
            .variables
            .filter
            .as_ref()
            .is_some_and(|filter| !filter.is_object())
        {
            return Err(String::from(
                "operationSignature.variables.filter must be an object",
            ));
        }
        if self
            .variables
            .order_by
            .as_ref()
            .is_some_and(|order_by| !order_by.is_object() && !order_by.is_array())
        {
            return Err(String::from(
                "operationSignature.variables.orderBy must be an object or array",
            ));
        }
        if self
            .variables
            .limit
            .is_some_and(|limit| limit == 0 || limit as usize > QUERY_MAX_RECORDS)
        {
            return Err(String::from(
                "operationSignature.variables.limit must be between 1 and 200",
            ));
        }
        if self
            .variables
            .cursor_filter
            .as_ref()
            .is_some_and(|cursor_filter| cursor_filter.cursor.trim().is_empty())
        {
            return Err(String::from(
                "operationSignature.variables.cursorFilter.cursor is required",
            ));
        }
        if self
            .fields
            .as_ref()
            .is_some_and(|fields| !valid_signature_fields(&Value::Object(fields.clone())))
        {
            return Err(String::from(
                "operationSignature.fields must be a nested field selection object",
            ));
        }
        Ok(())
    }

    pub fn matches_record(
        &self,
        action: &str,
        object_name_singular: &str,
        properties: &Value,
        schema: &dyn SnapshotFilterSchema,
    ) -> bool {
        if self.object_name_singular != object_name_singular {
            return false;
        }
        let before = properties.get("before");
        let after = properties.get("after");
        let ignore_deleted = matches!(action, "deleted" | "restored");
        let filter = self.variables.filter.clone().unwrap_or_else(|| json!({}));
        let matches = |record: &Value| {
            record_matches_filter(
                record,
                &filter,
                object_name_singular,
                schema,
                ignore_deleted,
            )
        };

        match action {
            "created" | "restored" => after.is_some_and(matches),
            "updated" => match (before, after) {
                (Some(before), Some(after)) if before.is_object() && after.is_object() => {
                    matches(before) || matches(after)
                }
                (Some(record), None) | (None, Some(record)) if record.is_object() => {
                    matches(record)
                }
                _ => false,
            },
            "deleted" => after.or(before).is_some_and(matches),
            "destroyed" => before.is_some_and(matches),
            _ => false,
        }
    }
}

fn valid_signature_fields(value: &Value) -> bool {
    match value {
        Value::Bool(true) => true,
        Value::Object(fields) => fields.values().all(valid_signature_fields),
        _ => false,
    }
}

pub(crate) fn actor_ids(actor: Option<&ActorContext>) -> (Value, Value) {
    let Some(actor) = actor else {
        return (Value::Null, Value::Null);
    };
    if actor.source == ActorSource::ApiKey {
        return (Value::Null, Value::Null);
    }
    (
        actor
            .user_id
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
        actor
            .workspace_member_id
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    )
}

pub fn record_mutation_properties(before: &Value, after: Option<&Value>) -> Option<Value> {
    let before = before.as_object()?;
    let after = after.and_then(Value::as_object);
    let mut keys = before.keys().cloned().collect::<Vec<_>>();
    if let Some(after) = after {
        keys.extend(after.keys().cloned());
    }
    keys.sort();
    keys.dedup();
    let mut updated_fields = Vec::new();
    let mut diff = Map::new();

    for key in keys {
        if matches!(key.as_str(), "updatedAt" | "searchVector") {
            continue;
        }
        let old = before.get(&key).unwrap_or(&Value::Null);
        let new = after
            .and_then(|after| after.get(&key))
            .unwrap_or(&Value::Null);
        if old == new {
            continue;
        }
        updated_fields.push(Value::String(key.clone()));
        diff.insert(key, json!({"before": old, "after": new}));
    }

    if updated_fields.is_empty() {
        return None;
    }

    Some(json!({
        "before": before,
        "after": after,
        "updatedFields": updated_fields,
        "diff": diff,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::jwt_service::TokenType;
    use crate::auth::middleware::AuthContext;
    use crate::metadata::flat::FieldMetadataType;
    use crate::query::filter::SnapshotFilterKey;
    use uuid::Uuid;

    fn auth(user_id: Uuid) -> AuthContext {
        AuthContext {
            token_type: TokenType::Access,
            user_id: Some(user_id),
            workspace_id: Some(Uuid::new_v4()),
            user_workspace_id: None,
            workspace_member_id: None,
            api_key_id: None,
            agent_id: None,
            auth_provider: None,
            is_impersonating: false,
        }
    }

    fn signature(filter: Value) -> RecordOperationSignature {
        RecordOperationSignature::parse(json!({
            "objectNameSingular": "person",
            "variables": {"filter": filter},
            "fields": {"id": true, "name": {"firstName": true}}
        }))
        .expect("signature")
    }

    struct TestSchema;

    impl SnapshotFilterSchema for TestSchema {
        fn snapshot_filter_key(&self, object_name: &str, key: &str) -> Option<SnapshotFilterKey> {
            if object_name != "person" {
                return None;
            }
            match key {
                "name" => Some(SnapshotFilterKey::Composite {
                    sub_fields: vec![crate::query::filter::SnapshotCompositeSubField {
                        name: String::from("firstName"),
                        field_type: FieldMetadataType::Text,
                    }],
                }),
                "deletedAt" | "jobTitle" => Some(SnapshotFilterKey::Scalar {
                    field_type: if key == "deletedAt" {
                        FieldMetadataType::DateTime
                    } else {
                        FieldMetadataType::Text
                    },
                    path: vec![String::from(key)],
                }),
                "company" => Some(SnapshotFilterKey::Relation),
                _ => None,
            }
        }
    }

    #[test]
    fn record_operation_signatures_parse_camel_case_and_reject_other_shapes() {
        let parsed = signature(json!({"name": {"firstName": {"eq": "Ada"}}})).clone();
        assert_eq!(parsed.object_name_singular, "person");
        assert_eq!(
            parsed.variables.filter,
            Some(json!({"name": {"firstName": {"eq": "Ada"}}}))
        );
        assert_eq!(parsed.fields.unwrap()["name"]["firstName"], true);
        assert!(RecordOperationSignature::parse(json!("FindManyPeople")).is_err());
        assert!(RecordOperationSignature::parse(json!({
            "objectNameSingular": "person",
            "variables": {"filter": []}
        }))
        .is_err());
        assert!(RecordOperationSignature::parse(json!({
            "objectNameSingular": "person",
            "variables": {"limit": 201}
        }))
        .is_err());
        assert!(RecordOperationSignature::parse(json!({
            "type": "metadata",
            "metadataName": "objectMetadata",
            "variables": {}
        }))
        .is_err());
    }

    #[test]
    fn cursor_filters_are_deserialized_and_validated() {
        let parsed = RecordOperationSignature::parse(json!({
            "objectNameSingular": "person",
            "variables": {
                "limit": 60,
                "cursorFilter": {"cursor": "abc", "cursorDirection": "after"}
            }
        }))
        .expect("cursor signature");
        assert_eq!(parsed.variables.limit, Some(60));
        assert_eq!(
            parsed.variables.cursor_filter,
            Some(RecordCursorFilter {
                cursor: String::from("abc"),
                cursor_direction: RecordCursorDirection::After,
            })
        );
        assert!(RecordOperationSignature::parse(json!({
            "objectNameSingular": "person",
            "variables": {"cursorFilter": {"cursor": "abc", "cursorDirection": "sideways"}}
        }))
        .is_err());
    }

    #[test]
    fn record_events_only_reach_exact_object_and_filter_matching_queries() {
        let registry = crate::events::EventStreamRegistry::default();
        let workspace_id = Uuid::new_v4();
        let other_workspace_id = Uuid::new_v4();
        let user = auth(Uuid::new_v4());
        let mut receiver = registry
            .open(workspace_id, String::from("stream-1"), &user)
            .expect("stream");
        let mut other_receiver = registry
            .open(other_workspace_id, String::from("stream-2"), &user)
            .expect("other stream");
        assert!(registry.add_query(
            workspace_id,
            "stream-1",
            &user,
            String::from("people-query"),
            signature(json!({"name": {"firstName": {"eq": "Ada"}}})),
        ));
        assert!(registry.add_query(
            workspace_id,
            "stream-1",
            &user,
            String::from("other-object-query"),
            RecordOperationSignature::parse(json!({
                "objectNameSingular": "company",
                "variables": {}
            }))
            .unwrap(),
        ));
        assert!(registry.add_query(
            other_workspace_id,
            "stream-2",
            &user,
            String::from("people-query"),
            signature(json!({})),
        ));

        registry.publish_record(
            workspace_id,
            Uuid::new_v4(),
            "updated",
            "person",
            "person-1",
            json!({
                "before": {"name": {"firstName": "Ada"}},
                "after": {"name": {"firstName": "Ada"}}
            }),
            &TestSchema,
            None,
        );

        let payload = receiver.try_recv().expect("matching event");
        assert_eq!(payload["eventStreamId"], "stream-1");
        assert_eq!(
            payload["objectRecordEventsWithQueryIds"][0]["queryIds"],
            json!(["people-query"])
        );
        assert!(other_receiver.try_recv().is_err());
        assert!(!registry.add_query(
            workspace_id,
            "stream-1",
            &auth(Uuid::new_v4()),
            String::from("other-query"),
            signature(json!({})),
        ));
        assert!(registry.remove_query(workspace_id, "stream-1", &user, "people-query"));
        registry.publish_record(
            workspace_id,
            Uuid::new_v4(),
            "updated",
            "person",
            "person-1",
            json!({}),
            &TestSchema,
            None,
        );
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn updates_match_before_or_after_entry_and_exit() {
        let ada = signature(json!({"name": {"firstName": {"eq": "Ada"}}}));
        let properties = json!({
            "before": {"name": {"firstName": "Grace"}},
            "after": {"name": {"firstName": "Ada"}}
        });
        assert!(ada.matches_record("updated", "person", &properties, &TestSchema));
        let properties = json!({
            "before": {"name": {"firstName": "Ada"}},
            "after": {"name": {"firstName": "Grace"}}
        });
        assert!(ada.matches_record("updated", "person", &properties, &TestSchema));
        let properties = json!({
            "before": {"name": {"firstName": "Grace"}},
            "after": {"name": {"firstName": "Grace"}}
        });
        assert!(!ada.matches_record("updated", "person", &properties, &TestSchema));
        assert!(!ada.matches_record("updated", "companies", &properties, &TestSchema));
    }

    #[test]
    fn soft_delete_ignores_only_implicit_deleted_filtering() {
        let all = signature(json!({}));
        let live = signature(json!({"deletedAt": {"is": "NULL"}}));
        let deleted = json!({
            "before": {"deletedAt": null},
            "after": {"deletedAt": "2026-01-01T00:00:00.000Z"}
        });
        assert!(all.matches_record("deleted", "person", &deleted, &TestSchema));
        assert!(!live.matches_record("deleted", "person", &deleted, &TestSchema));
        assert!(all.matches_record("updated", "person", &deleted, &TestSchema));
    }

    #[test]
    fn relation_filters_do_not_publish_events() {
        let registry = crate::events::EventStreamRegistry::default();
        let workspace_id = Uuid::new_v4();
        let user = auth(Uuid::new_v4());
        let mut receiver = registry
            .open(workspace_id, String::from("stream"), &user)
            .expect("stream");
        assert!(registry.add_query(
            workspace_id,
            "stream",
            &user,
            "query".into(),
            signature(json!({"or": [
                {"jobTitle": {"eq": "Engineer"}},
                {"company": {"name": {"eq": "Acme"}}}
            ]})),
        ));
        registry.publish_record(
            workspace_id,
            Uuid::new_v4(),
            "updated",
            "person",
            "person-1",
            json!({
                "before": {"jobTitle": "Engineer"},
                "after": {"jobTitle": "Engineer"}
            }),
            &TestSchema,
            None,
        );
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn mutation_payloads_diff_changed_fields_and_suppress_noops() {
        let before = json!({
            "name": "Ada",
            "jobTitle": "Engineer",
            "updatedAt": "2026-01-01T00:00:00.000Z",
            "searchVector": "old"
        });
        let after = json!({
            "name": "Ada Lovelace",
            "jobTitle": "Engineer",
            "updatedAt": "2026-01-02T00:00:00.000Z",
            "searchVector": "new"
        });
        let properties = record_mutation_properties(&before, Some(&after)).expect("changed");
        assert_eq!(properties["updatedFields"], json!(["name"]));
        assert_eq!(properties["diff"]["name"]["before"], "Ada");
        assert_eq!(properties["diff"]["name"]["after"], "Ada Lovelace");
        assert!(properties["diff"].get("updatedAt").is_none());
        assert!(properties["diff"].get("searchVector").is_none());
        assert!(record_mutation_properties(&before, Some(&before)).is_none());
    }

    #[test]
    fn writer_actor_ids_are_preserved_and_api_keys_stay_null() {
        let user_id = Uuid::new_v4();
        let member_id = Uuid::new_v4();
        let mut manual = ActorContext::manual(Some(member_id.to_string()), None);
        manual.user_id = Some(user_id.to_string());
        assert_eq!(
            actor_ids(Some(&manual)),
            (json!(user_id.to_string()), json!(member_id.to_string()))
        );
        let mut api_key = manual.clone();
        api_key.source = ActorSource::ApiKey;
        assert_eq!(actor_ids(Some(&api_key)), (Value::Null, Value::Null));
    }
}
