use crate::client::Endpoint;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QueryKey {
    pub operation: String,
    pub variables: String,
    pub endpoint: Endpoint,
}

impl QueryKey {
    pub fn new(
        operation: impl Into<String>,
        endpoint: Endpoint,
        variables: Option<&Value>,
    ) -> Self {
        Self {
            operation: operation.into(),
            variables: variables
                .map(|value| serde_json::to_string(&canonicalize(value)).unwrap_or_default())
                .unwrap_or_else(|| "{}".to_string()),
            endpoint,
        }
    }
}

fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Object(entries) => {
            let sorted = entries
                .iter()
                .collect::<Vec<_>>()
                .into_iter()
                .collect::<std::collections::BTreeMap<_, _>>();
            Value::Object(
                sorted
                    .into_iter()
                    .map(|(key, value)| (key.clone(), canonicalize(value)))
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize).collect()),
        _ => value.clone(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryStatus {
    Idle,
    Loading,
    Success,
    Error,
}

#[derive(Debug, Clone)]
pub struct QueryEntry {
    pub data: Value,
    pub status: QueryStatus,
    pub updated_at_ms: u128,
    pub subscription_ids: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct QueryCache {
    entries: HashMap<QueryKey, QueryEntry>,
}

impl QueryCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read(&self, key: &QueryKey) -> Option<&QueryEntry> {
        self.entries.get(key)
    }

    pub fn set_loading(&mut self, key: QueryKey) {
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.status = QueryStatus::Loading;
        } else {
            self.entries.insert(
                key,
                QueryEntry {
                    data: Value::Null,
                    status: QueryStatus::Loading,
                    updated_at_ms: now_ms(),
                    subscription_ids: Vec::new(),
                },
            );
        }
    }

    pub fn set_error(&mut self, key: &QueryKey) {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.status = QueryStatus::Error;
        }
    }

    pub fn insert(&mut self, key: QueryKey, data: Value) {
        self.entries.insert(
            key,
            QueryEntry {
                data,
                status: QueryStatus::Success,
                updated_at_ms: now_ms(),
                subscription_ids: Vec::new(),
            },
        );
    }

    pub fn subscribe(&mut self, key: &QueryKey, subscription_id: impl Into<String>) {
        if let Some(entry) = self.entries.get_mut(key) {
            let subscription_id = subscription_id.into();
            if !entry.subscription_ids.contains(&subscription_id) {
                entry.subscription_ids.push(subscription_id);
            }
        }
    }

    pub fn unsubscribe(&mut self, key: &QueryKey, subscription_id: &str) {
        if let Some(entry) = self.entries.get_mut(key) {
            entry
                .subscription_ids
                .retain(|current| current != subscription_id);
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = &QueryKey> {
        self.entries.keys()
    }

    pub fn get_mut(&mut self, key: &QueryKey) -> Option<&mut QueryEntry> {
        self.entries.get_mut(key)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecordId {
    pub object_name: String,
    pub id: String,
}

impl RecordId {
    pub fn new(object_name: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            object_name: object_name.into(),
            id: id.into(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RecordIndex {
    records: HashMap<RecordId, Value>,
    references: HashMap<QueryKey, Vec<RecordId>>,
}

impl RecordIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn index_records(&mut self, object_name: &str, records: &[Value]) {
        for record in records {
            if let Some(id) = record.get("id").and_then(Value::as_str) {
                self.records
                    .insert(RecordId::new(object_name, id), record.clone());
            }
        }
    }

    pub fn index_response(&mut self, object_name: &str, data: &Value) {
        for record in root_records(data) {
            self.insert_record(object_name, record);
        }
    }

    pub fn bind_query(&mut self, key: QueryKey, object_name: &str, data: &Value) {
        let ids = root_records(data)
            .into_iter()
            .filter_map(|record| {
                record
                    .get("id")
                    .and_then(Value::as_str)
                    .map(|id| RecordId::new(object_name, id))
            })
            .collect::<Vec<_>>();
        self.references.insert(key, ids);
        self.index_response(object_name, data);
    }

    pub fn get(&self, object_name: &str, id: &str) -> Option<&Value> {
        self.records.get(&RecordId::new(object_name, id))
    }

    pub fn queries_referencing(&self, object_name: &str, id: &str) -> Vec<QueryKey> {
        let record_id = RecordId::new(object_name, id);
        self.references
            .iter()
            .filter(|(_, records)| records.contains(&record_id))
            .map(|(key, _)| key.clone())
            .collect()
    }

    pub fn insert_record(&mut self, object_name: &str, record: Value) {
        if let Some(id) = record.get("id").and_then(Value::as_str) {
            self.records.insert(RecordId::new(object_name, id), record);
        }
    }
}

fn root_records(data: &Value) -> Vec<Value> {
    let mut records = Vec::new();
    let data = data.get("data").unwrap_or(data);
    let Some(data) = data.as_object() else {
        return records;
    };

    for value in data.values() {
        if let Some(record) = value.as_object() {
            if record.get("id").and_then(Value::as_str).is_some() {
                records.push(Value::Object(record.clone()));
                continue;
            }

            if let Some(edges) = record.get("edges").and_then(Value::as_array) {
                for edge in edges {
                    if let Some(node) = edge.get("node") {
                        if node.get("id").and_then(Value::as_str).is_some() {
                            records.push(node.clone());
                        }
                    }
                }
            }
        }

        if let Some(items) = value.as_array() {
            for item in items {
                if item.get("id").and_then(Value::as_str).is_some() {
                    records.push(item.clone());
                }
            }
        }
    }

    records
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn connection() -> Value {
        json!({
            "data": {
                "people": {
                    "edges": [
                        {"node": {"id": "p1", "name": {"firstName": "Ada"}}},
                        {"node": {"id": "p2", "name": {"firstName": "Grace"}}}
                    ]
                }
            }
        })
    }

    #[test]
    fn query_key_normalizes_variables_order_for_the_upstream_query_cache() {
        let left = QueryKey::new(
            "FindManyPeople",
            Endpoint::Core,
            Some(&json!({"first": 2, "filter": {"and": []}})),
        );
        let right = QueryKey::new(
            "FindManyPeople",
            Endpoint::Core,
            Some(&json!({"filter": {"and": []}, "first": 2})),
        );
        assert_eq!(left, right);
    }

    #[test]
    fn cached_connection_survives_a_loading_refetch() {
        let mut cache = QueryCache::new();
        let key = QueryKey::new("FindManyPeople", Endpoint::Core, None);
        cache.insert(key.clone(), connection());
        cache.set_loading(key.clone());
        assert_eq!(cache.read(&key).unwrap().status, QueryStatus::Loading);
        assert_eq!(
            cache.read(&key).unwrap().data["data"]["people"]["edges"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn record_index_finds_every_query_referencing_a_record() {
        let mut index = RecordIndex::new();
        let first = QueryKey::new("FindManyPeople", Endpoint::Core, None);
        let second = QueryKey::new("FindManyCompanies", Endpoint::Core, None);
        index.bind_query(first.clone(), "person", &connection());
        index.bind_query(
            second.clone(),
            "company",
            &json!({"data": {"companies": {"edges": []}}}),
        );
        assert_eq!(index.queries_referencing("person", "p1"), vec![first]);
        assert!(index.queries_referencing("company", "p1").is_empty());
        assert_eq!(
            index.get("person", "p2").unwrap()["name"]["firstName"],
            "Grace"
        );
    }

    #[test]
    fn record_index_handles_singular_mutation_results() {
        let mut index = RecordIndex::new();
        index.index_response(
            "person",
            &json!({"data": {"createPerson": {"id": "new", "name": {"firstName": "New"}}}}),
        );
        assert!(index.get("person", "new").is_some());
    }
}
