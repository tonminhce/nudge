use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Endpoint {
    Metadata,
    Core,
}

impl Endpoint {
    pub fn path(self) -> &'static str {
        match self {
            Self::Metadata => "/metadata",
            Self::Core => "/graphql",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GraphQlLocation {
    pub line: Option<u32>,
    pub column: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphQlError {
    pub message: String,
    #[serde(default)]
    pub locations: Vec<GraphQlLocation>,
    #[serde(default)]
    pub extensions: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GraphQlResponse {
    #[serde(default)]
    pub data: Option<Value>,
    #[serde(default)]
    pub errors: Option<Vec<GraphQlError>>,
    #[serde(default)]
    pub extensions: Option<Value>,
}

impl GraphQlResponse {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ClientError> {
        serde_json::from_slice(bytes).map_err(ClientError::Json)
    }

    pub fn into_data(self) -> Result<Value, ClientError> {
        if let Some(errors) = self.errors.filter(|errors| !errors.is_empty()) {
            return Err(ClientError::GraphQl(errors));
        }

        self.data.ok_or(ClientError::MissingData)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[cfg(not(target_arch = "wasm32"))]
    #[error("request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("invalid response JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("graphql returned {} error(s): {}", .0.len(), .0.iter().map(|error| error.message.as_str()).collect::<Vec<_>>().join("; "))]
    GraphQl(Vec<GraphQlError>),
    #[error("graphql response did not contain data")]
    MissingData,
    #[error("transport failed: {0}")]
    Transport(String),
    #[error("server URL must be an absolute http(s) URL")]
    InvalidBaseUrl,
}

#[derive(Debug, Clone)]
pub struct RequestOptions {
    pub endpoint: Endpoint,
    pub operation_name: Option<String>,
    pub variables: Option<Value>,
    pub bearer_token: Option<String>,
    pub session_cookie: Option<String>,
}

impl RequestOptions {
    pub fn new(endpoint: Endpoint) -> Self {
        Self {
            endpoint,
            operation_name: None,
            variables: None,
            bearer_token: None,
            session_cookie: None,
        }
    }

    pub fn operation_name(mut self, operation_name: impl Into<String>) -> Self {
        self.operation_name = Some(operation_name.into());
        self
    }

    pub fn variables(mut self, variables: Value) -> Self {
        self.variables = Some(variables);
        self
    }

    pub fn bearer_token(mut self, token: impl Into<String>) -> Self {
        self.bearer_token = Some(token.into());
        self
    }

    pub fn session_cookie(mut self, cookie: impl Into<String>) -> Self {
        self.session_cookie = Some(cookie.into());
        self
    }
}

#[async_trait::async_trait(?Send)]
pub trait GraphQLClient {
    async fn execute(
        &self,
        base_url: &str,
        query: &str,
        options: RequestOptions,
    ) -> Result<GraphQlResponse, ClientError>;

    async fn execute_data(
        &self,
        base_url: &str,
        query: &str,
        options: RequestOptions,
    ) -> Result<Value, ClientError> {
        self.execute(base_url, query, options).await?.into_data()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub struct ReqwestGraphQLClient {
    http: reqwest::Client,
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for ReqwestGraphQLClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl ReqwestGraphQLClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("reqwest client"),
        }
    }

    pub fn with_client(http: reqwest::Client) -> Self {
        Self { http }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait::async_trait(?Send)]
impl GraphQLClient for ReqwestGraphQLClient {
    async fn execute(
        &self,
        base_url: &str,
        query: &str,
        options: RequestOptions,
    ) -> Result<GraphQlResponse, ClientError> {
        if !(base_url.starts_with("http://") || base_url.starts_with("https://")) {
            return Err(ClientError::InvalidBaseUrl);
        }

        let url = format!(
            "{}{}",
            base_url.trim_end_matches('/'),
            options.endpoint.path()
        );
        let mut body = serde_json::Map::new();
        body.insert(String::from("query"), Value::String(query.to_string()));
        body.insert(
            String::from("variables"),
            options
                .variables
                .unwrap_or_else(|| Value::Object(Map::new())),
        );
        if let Some(operation_name) = options.operation_name {
            body.insert(String::from("operationName"), Value::String(operation_name));
        }

        let mut request = self.http.post(url).json(&body);

        if let Some(token) = options.bearer_token {
            request = request.bearer_auth(token);
        }

        if let Some(cookie) = options.session_cookie {
            request = request.header(reqwest::header::COOKIE, cookie);
        }

        let response = self
            .http
            .execute(request.build()?)
            .await?
            .error_for_status()?;
        let bytes = response.bytes().await?;

        GraphQlResponse::from_bytes(&bytes)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub type BrowserGraphQLClient = ReqwestGraphQLClient;

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Default)]
pub struct BrowserGraphQLClient;

#[cfg(target_arch = "wasm32")]
#[async_trait::async_trait(?Send)]
impl GraphQLClient for BrowserGraphQLClient {
    async fn execute(
        &self,
        base_url: &str,
        query: &str,
        options: RequestOptions,
    ) -> Result<GraphQlResponse, ClientError> {
        if !(base_url.starts_with("http://") || base_url.starts_with("https://")) {
            return Err(ClientError::InvalidBaseUrl);
        }

        let url = format!(
            "{}{}",
            base_url.trim_end_matches('/'),
            options.endpoint.path()
        );
        let mut body = serde_json::Map::new();
        body.insert(String::from("query"), Value::String(query.to_string()));
        body.insert(
            String::from("variables"),
            options
                .variables
                .unwrap_or_else(|| Value::Object(Map::new())),
        );
        if let Some(operation_name) = options.operation_name {
            body.insert(String::from("operationName"), Value::String(operation_name));
        }

        let mut builder = gloo_net::http::Request::post(&url);
        if let Some(token) = options.bearer_token {
            builder = builder.header("authorization", &format!("Bearer {token}"));
        }
        let _ = options.session_cookie;
        let request = builder
            .credentials(web_sys::RequestCredentials::Include)
            .json(&body)
            .map_err(|error| ClientError::Transport(error.to_string()))?;
        let response = request
            .send()
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))?;
        let status = response.status();
        if !(200..300).contains(&status) {
            return Err(ClientError::Transport(format!("HTTP {status}")));
        }
        let text = response
            .text()
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))?;
        GraphQlResponse::from_bytes(text.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graphql_response_matches_the_upstream_error_envelope() {
        let response = GraphQlResponse::from_bytes(
            br#"{"data":{"people":null},"errors":[{"message":"Record not found","locations":[{"line":1,"column":10}],"extensions":{"code":"NOT_FOUND","subCode":"RECORD_NOT_FOUND"}}]}"#,
        )
        .unwrap();

        assert_eq!(
            response.errors.as_ref().unwrap()[0]
                .extensions
                .as_ref()
                .unwrap()["subCode"],
            "RECORD_NOT_FOUND"
        );
        assert!(matches!(response.into_data(), Err(ClientError::GraphQl(_))));
    }

    #[test]
    fn metadata_and_core_endpoints_keep_the_upstream_paths() {
        assert_eq!(Endpoint::Metadata.path(), "/metadata");
        assert_eq!(Endpoint::Core.path(), "/graphql");
    }

    #[test]
    fn request_options_keep_operation_and_variables() {
        let options = RequestOptions::new(Endpoint::Core)
            .operation_name("FindManyPeople")
            .variables(serde_json::json!({"first": 2}));
        assert_eq!(options.operation_name.as_deref(), Some("FindManyPeople"));
        assert_eq!(options.variables.unwrap()["first"], 2);
    }
}
