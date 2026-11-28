use std::fmt;

use reqwest::Url;

pub const MAX_REDIRECTS_MESSAGE: &str = "Maximum number of redirects exceeded.";

pub fn blocked_request_message(address: &str) -> String {
    format!("Request to internal IP address {address} is not allowed.")
}

pub fn blocked_connection_message(address: &str) -> String {
    format!("Connection to internal IP address {address} is not allowed.")
}

pub fn unvalidatable_address_message(address: &str) -> String {
    format!("Request to unvalidatable IP address {address} is not allowed.")
}

pub fn blocked_protocol_message(protocol: &str) -> String {
    format!("Protocol {protocol} is not allowed. Only HTTP and HTTPS are permitted.")
}

pub fn is_blocked_message(message: &str) -> bool {
    (message.contains("internal IP address") || message.contains("unvalidatable IP address"))
        && message.contains("is not allowed")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboundRequestSource {
    Webhook,
    WorkflowHttp,
    WorkflowHttpTest,
    AiHttpTool,
    LogicFunction,
    McpServer,
    ConnectionProvider,
}

impl OutboundRequestSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Webhook => "webhook",
            Self::WorkflowHttp => "workflow-http",
            Self::WorkflowHttpTest => "workflow-http-test",
            Self::AiHttpTool => "ai-http-tool",
            Self::LogicFunction => "logic-function",
            Self::McpServer => "mcp-server",
            Self::ConnectionProvider => "connection-provider",
        }
    }
}

impl fmt::Display for OutboundRequestSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboundRequestContext {
    pub workspace_id: String,
    pub source: OutboundRequestSource,
    pub user_id: Option<String>,
    pub user_workspace_id: Option<String>,
    pub workspace_member_id: Option<String>,
    pub workflow_run_id: Option<String>,
    pub workflow_step_id: Option<String>,
}

impl OutboundRequestContext {
    pub fn new(workspace_id: impl Into<String>, source: OutboundRequestSource) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            source,
            user_id: None,
            user_workspace_id: None,
            workspace_member_id: None,
            workflow_run_id: None,
            workflow_step_id: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboundRequestOutcome {
    Success,
    HttpError,
    Blocked,
    Timeout,
    NetworkError,
    OtherError,
}

impl OutboundRequestOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::HttpError => "http-error",
            Self::Blocked => "blocked",
            Self::Timeout => "timeout",
            Self::NetworkError => "network-error",
            Self::OtherError => "other-error",
        }
    }
}

impl fmt::Display for OutboundRequestOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboundRequestOutcomeDetails {
    pub outcome: OutboundRequestOutcome,
    pub status: Option<u16>,
    pub reason: Option<String>,
}

#[derive(Debug)]
pub enum SecureHttpError {
    Blocked(String),
    InvalidUrl(String),
    Lookup(String),
    InvalidHeader(String),
    MaxRedirects,
    PayloadTooLarge { limit: u64, declared: u64 },
    Transport(reqwest::Error),
}

impl SecureHttpError {
    pub fn blocked(message: impl Into<String>) -> Self {
        Self::Blocked(message.into())
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked(_))
    }

    pub fn message(&self) -> String {
        match self {
            Self::Blocked(message) => message.clone(),
            Self::InvalidUrl(url) => format!("Invalid URL {url}."),
            Self::Lookup(reason) | Self::InvalidHeader(reason) => reason.clone(),
            Self::MaxRedirects => MAX_REDIRECTS_MESSAGE.to_string(),
            Self::PayloadTooLarge { limit, declared } => {
                format!("Response of {declared} bytes exceeds the {limit} byte payload limit.")
            }
            Self::Transport(error) => describe_transport(error),
        }
    }

    pub fn status(&self) -> Option<u16> {
        match self {
            Self::Transport(error) => error.status().map(|status| status.as_u16()),
            _ => None,
        }
    }

    pub fn outcome(&self) -> OutboundRequestOutcome {
        match self {
            Self::Blocked(_) => OutboundRequestOutcome::Blocked,
            Self::Transport(error) => {
                if error.is_status() {
                    OutboundRequestOutcome::HttpError
                } else if error.is_timeout() {
                    OutboundRequestOutcome::Timeout
                } else if error.is_connect() || error.is_request() {
                    OutboundRequestOutcome::NetworkError
                } else {
                    OutboundRequestOutcome::OtherError
                }
            }
            Self::InvalidUrl(_) | Self::MaxRedirects => OutboundRequestOutcome::OtherError,
            Self::Lookup(_) | Self::PayloadTooLarge { .. } => OutboundRequestOutcome::NetworkError,
            Self::InvalidHeader(_) => OutboundRequestOutcome::OtherError,
        }
    }

    pub fn details(&self) -> OutboundRequestOutcomeDetails {
        let outcome = self.outcome();
        OutboundRequestOutcomeDetails {
            outcome,
            status: self.status(),
            reason: (outcome == OutboundRequestOutcome::Blocked).then(|| self.message()),
        }
    }
}

impl fmt::Display for SecureHttpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message())
    }
}

impl std::error::Error for SecureHttpError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transport(error) => Some(error),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for SecureHttpError {
    fn from(error: reqwest::Error) -> Self {
        match find_blocked_message(&error) {
            Some(message) => Self::Blocked(message),
            None => Self::Transport(error),
        }
    }
}

pub fn describe_outcome(status: u16) -> OutboundRequestOutcomeDetails {
    let outcome = if (200..300).contains(&status) {
        OutboundRequestOutcome::Success
    } else {
        OutboundRequestOutcome::HttpError
    };
    OutboundRequestOutcomeDetails {
        outcome,
        status: Some(status),
        reason: None,
    }
}

pub fn describe_transport(error: &reqwest::Error) -> String {
    let message = error.to_string();
    let mut cause: Option<String> = None;
    let mut source = std::error::Error::source(error);
    while let Some(current) = source {
        let text = current.to_string();
        if !text.is_empty() {
            cause = Some(text);
        }
        source = current.source();
    }
    match cause {
        Some(cause) => format!("{message}: {cause}"),
        None => message,
    }
}

pub fn find_blocked_message(error: &reqwest::Error) -> Option<String> {
    let display = error.to_string();
    if is_blocked_message(&display) {
        return Some(display);
    }
    let mut source = std::error::Error::source(error);
    while let Some(current) = source {
        let message = current.to_string();
        if is_blocked_message(&message) {
            return Some(message);
        }
        source = current.source();
    }
    None
}

pub fn redact_outbound_request_url(url: &Url) -> String {
    if url.scheme() == "http" || url.scheme() == "https" {
        return format!("{}{}", url.origin().ascii_serialization(), url.path());
    }
    format!("{}:", url.scheme())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocked_messages_carry_the_upstream_phrasing() {
        assert_eq!(
            blocked_request_message("127.0.0.1"),
            "Request to internal IP address 127.0.0.1 is not allowed."
        );
        assert_eq!(
            blocked_connection_message("169.254.169.254"),
            "Connection to internal IP address 169.254.169.254 is not allowed."
        );
        assert_eq!(
            unvalidatable_address_message("nonsense"),
            "Request to unvalidatable IP address nonsense is not allowed."
        );
        assert_eq!(
            blocked_protocol_message("file:"),
            "Protocol file: is not allowed. Only HTTP and HTTPS are permitted."
        );
        assert!(is_blocked_message(&blocked_request_message("10.0.0.1")));
        assert!(is_blocked_message(&blocked_connection_message("10.0.0.1")));
        assert!(!is_blocked_message("connection refused"));
    }

    #[test]
    fn error_outcome_classification() {
        let blocked = SecureHttpError::blocked(blocked_connection_message("127.0.0.1"));
        assert!(blocked.is_blocked());
        let details = blocked.details();
        assert_eq!(details.outcome, OutboundRequestOutcome::Blocked);
        assert_eq!(details.status, None);
        assert!(details.reason.is_some());
        assert_eq!(
            SecureHttpError::Lookup("no records".to_string()).outcome(),
            OutboundRequestOutcome::NetworkError
        );
    }

    #[test]
    fn urls_are_redacted_to_origin_and_path() {
        let signed = Url::parse("https://files.example.com/a/b.png?token=secret#frag")
            .expect("parse signed url");
        assert_eq!(
            redact_outbound_request_url(&signed),
            "https://files.example.com/a/b.png"
        );
        let other = Url::parse("ftp://host/path").expect("parse ftp url");
        assert_eq!(redact_outbound_request_url(&other), "ftp:");
    }

    #[test]
    fn source_and_outcome_labels_match_upstream() {
        assert_eq!(OutboundRequestSource::Webhook.as_str(), "webhook");
        assert_eq!(
            OutboundRequestSource::WorkflowHttpTest.as_str(),
            "workflow-http-test"
        );
        assert_eq!(OutboundRequestSource::AiHttpTool.as_str(), "ai-http-tool");
        assert_eq!(
            OutboundRequestSource::ConnectionProvider.as_str(),
            "connection-provider"
        );
        assert_eq!(OutboundRequestOutcome::HttpError.as_str(), "http-error");
        assert_eq!(
            OutboundRequestOutcome::NetworkError.as_str(),
            "network-error"
        );
        assert_eq!(OutboundRequestOutcome::OtherError.as_str(), "other-error");
    }
}
