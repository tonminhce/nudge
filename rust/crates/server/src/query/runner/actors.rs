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
    pub user_id: Option<String>,
    pub workspace_member_id: Option<String>,
    pub source: ActorSource,
    pub name: Option<String>,
    pub context: Value,
}

impl ActorContext {
    pub fn manual(workspace_member_id: Option<String>, name: Option<String>) -> Self {
        Self {
            user_id: None,
            workspace_member_id,
            source: ActorSource::Manual,
            name,
            context: Value::Object(serde_json::Map::new()),
        }
    }

    pub fn from_auth(auth: &crate::auth::middleware::AuthContext) -> Self {
        let is_api_key = auth.token_type == crate::auth::jwt_service::TokenType::ApiKey;
        Self {
            user_id: if is_api_key {
                None
            } else {
                auth.user_id.map(|id| id.to_string())
            },
            workspace_member_id: if is_api_key {
                None
            } else {
                auth.workspace_member_id.map(|id| id.to_string())
            },
            source: if is_api_key {
                ActorSource::ApiKey
            } else if auth.agent_id.is_some() {
                ActorSource::Agent
            } else {
                ActorSource::Manual
            },
            name: None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::jwt_service::TokenType;
    use uuid::Uuid;

    #[test]
    fn auth_actor_preserves_writer_identity_but_not_api_key_identity() {
        let user_id = Uuid::new_v4();
        let member_id = Uuid::new_v4();
        let auth = crate::auth::middleware::AuthContext {
            token_type: TokenType::Access,
            user_id: Some(user_id),
            workspace_id: Some(Uuid::new_v4()),
            user_workspace_id: None,
            workspace_member_id: Some(member_id),
            api_key_id: None,
            agent_id: None,
            auth_provider: None,
            is_impersonating: false,
        };
        let actor = ActorContext::from_auth(&auth);
        assert_eq!(actor.user_id, Some(user_id.to_string()));
        assert_eq!(actor.workspace_member_id, Some(member_id.to_string()));
        assert_eq!(actor.source, ActorSource::Manual);

        let api_key = crate::auth::middleware::AuthContext {
            token_type: TokenType::ApiKey,
            user_id: Some(user_id),
            workspace_id: Some(Uuid::new_v4()),
            user_workspace_id: None,
            workspace_member_id: Some(member_id),
            api_key_id: Some(Uuid::new_v4()),
            agent_id: None,
            auth_provider: None,
            is_impersonating: false,
        };
        let actor = ActorContext::from_auth(&api_key);
        assert_eq!(actor.user_id, None);
        assert_eq!(actor.workspace_member_id, None);
        assert_eq!(actor.source, ActorSource::ApiKey);
    }
}
