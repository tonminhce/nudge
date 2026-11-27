use crate::query_cache::QueryCache;
use crate::query_cache::QueryKey;
use crate::query_cache::RecordIndex;
use serde_json::Map;
use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct OptimisticEngine {
    pub cache: QueryCache,
    pub records: RecordIndex,
}

impl OptimisticEngine {
    pub fn new(cache: QueryCache, records: RecordIndex) -> Self {
        Self { cache, records }
    }

    pub fn edit_field(
        &mut self,
        object_name: &str,
        record_id: &str,
        field_name: &str,
        value: Value,
    ) -> OptimisticSnapshot {
        let before_record = self.records.get(object_name, record_id).cloned();
        let mut updated = before_record
            .clone()
            .unwrap_or_else(|| serde_json::json!({"id": record_id}));
        set_field(&mut updated, field_name, value.clone());
        self.records.insert_record(object_name, updated);

        let keys = self.records.queries_referencing(object_name, record_id);
        let mut before_queries = Vec::with_capacity(keys.len());

        for key in keys {
            if let Some(entry) = self.cache.get_mut(&key) {
                before_queries.push((key.clone(), entry.data.clone()));
                patch_record_field(&mut entry.data, record_id, field_name, &value);
                entry.status = crate::query_cache::QueryStatus::Success;
            }
        }

        OptimisticSnapshot {
            object_name: object_name.to_string(),
            before_record,
            before_queries,
        }
    }

    pub fn attach_relation(
        &mut self,
        object_name: &str,
        record_id: &str,
        relation_field: &str,
        target_record: Value,
    ) -> OptimisticSnapshot {
        self.edit_field(object_name, record_id, relation_field, target_record)
    }

    pub fn detach_relation(
        &mut self,
        object_name: &str,
        record_id: &str,
        relation_field: &str,
    ) -> OptimisticSnapshot {
        self.edit_field(object_name, record_id, relation_field, Value::Null)
    }

    pub fn rollback(&mut self, snapshot: OptimisticSnapshot) {
        for (key, data) in snapshot.before_queries {
            self.cache.insert(key, data);
        }

        if let Some(record) = snapshot.before_record {
            self.records.insert_record(&snapshot.object_name, record);
        }
    }
}

#[derive(Debug, Clone)]
pub struct OptimisticSnapshot {
    object_name: String,
    before_record: Option<Value>,
    before_queries: Vec<(QueryKey, Value)>,
}

fn set_field(record: &mut Value, field_name: &str, value: Value) {
    let Some(object) = record.as_object_mut() else {
        return;
    };

    let mut segments = field_name.split('.').peekable();
    let first = segments.next().unwrap_or(field_name);
    if segments.peek().is_none() {
        object.insert(first.to_string(), value);
        return;
    }

    let child = object
        .entry(first.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    set_nested(child, segments, value);
}

fn set_nested<'a>(
    value: &mut Value,
    mut segments: impl Iterator<Item = &'a str>,
    new_value: Value,
) {
    let Some(segment) = segments.next() else {
        *value = new_value;
        return;
    };
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
    let Some(object) = value.as_object_mut() else {
        return;
    };
    let child = object
        .entry(segment.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    set_nested(child, segments, new_value);
}

fn patch_record_field(data: &mut Value, record_id: &str, field_name: &str, value: &Value) {
    let has_data = data.get("data").is_some();
    let payload = if has_data {
        data.get_mut("data").expect("data key checked")
    } else {
        data
    };
    let Some(payload) = payload.as_object_mut() else {
        return;
    };

    for root in payload.values_mut() {
        if let Some(edges) = root
            .as_object_mut()
            .and_then(|object| object.get_mut("edges"))
            .and_then(Value::as_array_mut)
        {
            for edge in edges {
                if let Some(node) = edge.get_mut("node") {
                    if node.get("id").and_then(Value::as_str) == Some(record_id) {
                        set_field(node, field_name, value.clone());
                    }
                }
            }
        }

        if root.get("id").and_then(Value::as_str) == Some(record_id) {
            set_field(root, field_name, value.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::Endpoint;
    use serde_json::json;

    fn engine() -> OptimisticEngine {
        let mut cache = QueryCache::new();
        let mut records = RecordIndex::new();
        let key = QueryKey::new("FindManyPeople", Endpoint::Core, None);
        let data = json!({
            "data": {
                "people": {
                    "edges": [{"node": {"id": "p1", "name": {"firstName": "Ada", "lastName": "Lovelace"}}}]
                }
            }
        });
        records.bind_query(key.clone(), "person", &data);
        cache.insert(key, data);
        OptimisticEngine::new(cache, records)
    }

    #[test]
    fn optimistic_edit_patches_the_record_index_and_every_referencing_query() {
        let mut engine = engine();
        let snapshot = engine.edit_field("person", "p1", "jobTitle", json!("Engineer"));
        assert_eq!(
            engine.records.get("person", "p1").unwrap()["jobTitle"],
            "Engineer"
        );
        let key = QueryKey::new("FindManyPeople", Endpoint::Core, None);
        assert_eq!(
            engine.cache.read(&key).unwrap().data["data"]["people"]["edges"][0]["node"]["jobTitle"],
            "Engineer"
        );
        engine.rollback(snapshot);
        assert!(engine
            .records
            .get("person", "p1")
            .unwrap()
            .get("jobTitle")
            .is_none());
    }

    #[test]
    fn nested_composite_field_edit_preserves_sibling_values() {
        let mut engine = engine();
        let snapshot = engine.edit_field("person", "p1", "name.firstName", json!("Augusta"));
        assert_eq!(
            engine.records.get("person", "p1").unwrap()["name"]["lastName"],
            "Lovelace"
        );
        assert_eq!(
            engine.records.get("person", "p1").unwrap()["name"]["firstName"],
            "Augusta"
        );
        engine.rollback(snapshot);
        assert_eq!(
            engine.records.get("person", "p1").unwrap()["name"]["firstName"],
            "Ada"
        );
    }

    #[test]
    fn relation_attach_and_detach_follow_the_upstream_optimistic_effect() {
        let mut engine = engine();
        let attached = engine.attach_relation(
            "person",
            "p1",
            "company",
            json!({"id": "c1", "name": "Analytical Engines"}),
        );
        assert_eq!(
            engine.records.get("person", "p1").unwrap()["company"]["id"],
            "c1"
        );
        engine.rollback(attached);
        let detached = engine.detach_relation("person", "p1", "company");
        assert!(engine.records.get("person", "p1").unwrap()["company"].is_null());
        engine.rollback(detached);
    }
}
