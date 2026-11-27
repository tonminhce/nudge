use serde_json::Value;

use super::base::ObjectModel;
use super::RunnerError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorSource {
    Manual,
    ApiKey,
    Application,
    Agent,
}

impl ActorSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "MANUAL",
            Self::ApiKey => "API_KEY",
            Self::Application => "APPLICATION",
            Self::Agent => "AGENT",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        let source = match value {
            "MANUAL" => Self::Manual,
            "API_KEY" => Self::ApiKey,
            "APPLICATION" => Self::Application,
            "AGENT" => Self::Agent,
            _ => return None,
        };

        Some(source)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorContext {
    pub workspace_member_id: Option<String>,
    pub source: ActorSource,
    pub name: Option<String>,
    pub context: Value,
}

impl ActorContext {
    pub fn manual(workspace_member_id: Option<String>, name: Option<String>) -> Self {
        Self {
            workspace_member_id,
            source: ActorSource::Manual,
            name,
            context: Value::Object(serde_json::Map::new()),
        }
    }

    pub fn to_value(&self) -> Value {
        let mut actor = serde_json::Map::new();

        if let Some(id) = &self.workspace_member_id {
            actor.insert(String::from("workspaceMemberId"), Value::String(id.clone()));
        } else {
            actor.insert(String::from("workspaceMemberId"), Value::Null);
        }

        actor.insert(
            String::from("source"),
            Value::String(self.source.as_str().to_string()),
        );
        actor.insert(
            String::from("name"),
            self.name.clone().map(Value::String).unwrap_or(Value::Null),
        );
        actor.insert(String::from("context"), self.context.clone());
        Value::Object(actor)
    }
}

pub fn inject_actor_fields_on_create(
    model: &ObjectModel,
    record: &mut serde_json::Map<String, Value>,
    actor: Option<&ActorContext>,
) -> Result<(), RunnerError> {
    let Some(actor) = actor else {
        return Ok(());
    };

    if model.fields_by_name.contains_key("createdBy") {
        inject_actor_field(record, "createdBy", actor);
    }

    if model.fields_by_name.contains_key("updatedBy") {
        inject_actor_field(record, "updatedBy", actor);
    }

    Ok(())
}

pub fn inject_updated_by_on_update(
    model: &ObjectModel,
    record: &mut serde_json::Map<String, Value>,
    actor: Option<&ActorContext>,
) -> Result<(), RunnerError> {
    let Some(actor) = actor else {
        return Ok(());
    };

    if model.fields_by_name.contains_key("updatedBy") {
        inject_actor_field(record, "updatedBy", actor);
    }

    Ok(())
}

fn inject_actor_field(
    record: &mut serde_json::Map<String, Value>,
    field_name: &str,
    actor: &ActorContext,
) {
    if record
        .get(field_name)
        .is_some_and(|value| value.as_object().is_some_and(|object| !object.is_empty()))
    {
        return;
    }

    record.insert(field_name.to_string(), actor.to_value());
}
