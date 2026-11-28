use std::time::{Duration, SystemTime};

use reqwest::redirect::Policy;
use reqwest::Client;

use crate::webhook::signature::{
    generate_nonce, generate_signature, timestamp_millis, HEADER_NONCE, HEADER_SIGNATURE,
    HEADER_TIMESTAMP,
};

pub const WEBHOOK_REQUEST_TIMEOUT_MS: u64 = 5_000;
pub const WEBHOOK_MAX_REDIRECTS: usize = 5;
pub const SSRF_BLOCKED_ERROR_MESSAGE: &str =
    "Webhook URL resolves to a private/internal IP address";

pub fn is_ssrf_blocked_error(message: &str) -> bool {
    message.contains("internal IP address") && message.contains("is not allowed")
}

pub fn default_webhook_client() -> Client {
    Client::builder()
        .redirect(Policy::limited(WEBHOOK_MAX_REDIRECTS))
        .build()
        .expect("default webhook client can always be built")
}

fn has_absolute_protocol(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();

    lower.starts_with("http://") || lower.starts_with("https://")
}

pub fn ensure_absolute_url(value: &str) -> String {
    let trimmed = value.trim();

    if has_absolute_protocol(trimmed) {
        return trimmed.to_string();
    }

    format!("https://{}", trimmed)
}

#[derive(Clone, PartialEq, Eq)]
pub struct WebhookDeliveryRequest {
    pub target_url: String,
    pub webhook_id: String,
    pub event_name: String,
    pub secret: Option<String>,
    pub payload_json: String,
}

impl std::fmt::Debug for WebhookDeliveryRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebhookDeliveryRequest")
            .field("target_url", &self.target_url)
            .field("webhook_id", &self.webhook_id)
            .field("event_name", &self.event_name)
            .field("secret", &self.secret.as_ref().map(|_| "[redacted]"))
            .field("payload_json", &self.payload_json)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebhookDeliveryOutcome {
    pub url: String,
    pub webhook_id: String,
    pub event_name: String,
    pub status: Option<u16>,
    pub success: bool,
    pub error: Option<String>,
    pub ssrf_blocked: bool,
}

impl WebhookDeliveryOutcome {
    pub(crate) fn build(
        request: &WebhookDeliveryRequest,
        status: Option<u16>,
        error: Option<String>,
    ) -> Self {
        let success = matches!(status, Some(code) if (200..300).contains(&code));
        let ssrf_blocked = error.as_deref().is_some_and(is_ssrf_blocked_error);

        Self {
            url: request.target_url.clone(),
            webhook_id: request.webhook_id.clone(),
            event_name: request.event_name.clone(),
            status,
            success,
            error,
            ssrf_blocked,
        }
    }

    pub fn event_log_error(&self) -> Option<&str> {
        self.ssrf_blocked.then_some(SSRF_BLOCKED_ERROR_MESSAGE)
    }
}

pub async fn call_webhook(
    client: &Client,
    request: &WebhookDeliveryRequest,
) -> WebhookDeliveryOutcome {
    let mut builder = client
        .post(ensure_absolute_url(&request.target_url))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .timeout(Duration::from_millis(WEBHOOK_REQUEST_TIMEOUT_MS));

    if let Some(secret) = request
        .secret
        .as_deref()
        .filter(|secret| !secret.is_empty())
    {
        let timestamp = timestamp_millis(SystemTime::now());
        let signature = generate_signature(secret, &timestamp, &request.payload_json);

        builder = builder
            .header(HEADER_TIMESTAMP, timestamp)
            .header(HEADER_SIGNATURE, signature)
            .header(HEADER_NONCE, generate_nonce());
    }

    match builder.body(request.payload_json.clone()).send().await {
        Ok(response) => {
            let status = response.status().as_u16();

            WebhookDeliveryOutcome::build(request, Some(status), None)
        }
        Err(error) => {
            let status = error.status().map(|status| status.as_u16());

            WebhookDeliveryOutcome::build(request, status, Some(error.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    use tokio::net::TcpListener;

    use crate::webhook::signature::generate_webhook_secret;

    async fn read_full_request(socket: &mut tokio::net::TcpStream) -> String {
        let mut received: Vec<u8> = Vec::new();
        let mut buffer = [0u8; 4096];
        let header_end = loop {
            let read = socket.read(&mut buffer).await.unwrap();

            if read == 0 {
                break None;
            }

            received.extend_from_slice(&buffer[..read]);

            if let Some(index) = received.windows(4).position(|window| window == b"\r\n\r\n") {
                break Some(index + 4);
            }
        };

        let Some(end) = header_end else {
            return String::from_utf8_lossy(&received).into_owned();
        };

        let headers = String::from_utf8_lossy(&received[..end]).to_lowercase();
        let content_length = headers
            .lines()
            .find_map(|line| line.strip_prefix("content-length: "))
            .and_then(|value| value.trim().parse::<usize>().ok())
            .unwrap_or(0);

        while received.len() < end + content_length {
            let read = socket.read(&mut buffer).await.unwrap();

            if read == 0 {
                break;
            }

            received.extend_from_slice(&buffer[..read]);
        }

        String::from_utf8_lossy(&received).into_owned()
    }

    #[test]
    fn ssrf_recognition_matches_the_upstream_two_marker_test() {
        assert!(is_ssrf_blocked_error(
            "requested host resolves to an internal IP address, which is not allowed"
        ));
        assert!(!is_ssrf_blocked_error(
            "requested host resolves to an internal IP address"
        ));
        assert!(!is_ssrf_blocked_error("connection refused"));
        assert_eq!(
            WebhookDeliveryOutcome::build(
                &WebhookDeliveryRequest {
                    target_url: "http://localhost/hook".to_string(),
                    webhook_id: "w".to_string(),
                    event_name: "e".to_string(),
                    secret: None,
                    payload_json: "{}".to_string(),
                },
                None,
                Some("the internal IP address 10.0.0.1 is not allowed".to_string()),
            )
            .event_log_error(),
            Some(SSRF_BLOCKED_ERROR_MESSAGE)
        );
        assert_eq!(
            WebhookDeliveryOutcome::build(
                &WebhookDeliveryRequest {
                    target_url: "http://localhost/hook".to_string(),
                    webhook_id: "w".to_string(),
                    event_name: "e".to_string(),
                    secret: None,
                    payload_json: "{}".to_string(),
                },
                None,
                Some("dns resolution failed".to_string()),
            )
            .event_log_error(),
            None
        );
    }

    #[test]
    fn ensure_absolute_url_prefixes_missing_protocol() {
        assert_eq!(
            ensure_absolute_url("example.com/hook"),
            "https://example.com/hook"
        );
        assert_eq!(
            ensure_absolute_url(" https://example.com/hook "),
            "https://example.com/hook"
        );
        assert_eq!(
            ensure_absolute_url("http://example.com/hook"),
            "http://example.com/hook"
        );
        assert_eq!(
            ensure_absolute_url("HTTPS://example.com/hook"),
            "HTTPS://example.com/hook"
        );
    }

    #[tokio::test]
    async fn signed_delivery_posts_body_and_headers_to_local_listener() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let secret = generate_webhook_secret();
        let payload =
            "{\"targetUrl\":\"x\",\"eventName\":\"person.created\",\"record\":{\"name\":\"Ünicode\"}}"
                .to_string();

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let request = read_full_request(&mut socket).await;
            socket
                .write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n")
                .await
                .unwrap();

            request
        });

        let outcome = call_webhook(
            &default_webhook_client(),
            &WebhookDeliveryRequest {
                target_url: format!("http://{}/hook", addr),
                webhook_id: "3fa85f64-5717-4562-b3fc-2c963f66afa6".to_string(),
                event_name: "person.created".to_string(),
                secret: Some(secret.clone()),
                payload_json: payload.clone(),
            },
        )
        .await;

        assert!(outcome.success);
        assert_eq!(outcome.status, Some(204));
        assert_eq!(outcome.error, None);

        let raw = server.await.unwrap();
        let lower = raw.to_lowercase();
        let signature = lower
            .split("\r\n")
            .find_map(|line| line.strip_prefix("x-twenty-webhook-signature: "))
            .map(|value| value.trim().to_string())
            .expect("signature header");
        let timestamp = lower
            .split("\r\n")
            .find_map(|line| line.strip_prefix("x-twenty-webhook-timestamp: "))
            .map(|value| value.trim().to_string())
            .expect("timestamp header");
        let nonce = lower
            .split("\r\n")
            .find_map(|line| line.strip_prefix("x-twenty-webhook-nonce: "))
            .map(|value| value.trim().to_string())
            .expect("nonce header");
        let body = raw
            .split_once("\r\n\r\n")
            .map(|(_, body)| body.to_string())
            .expect("request body");

        assert_eq!(body, payload);
        assert_eq!(
            generate_signature(&secret, &timestamp, &body),
            signature,
            "the signature must validate against the exact delivered body"
        );
        assert!(lower.contains("content-type: application/json"));
        assert_eq!(nonce.len(), 32);
        assert!(timestamp.chars().all(|c| c.is_ascii_digit()));
    }

    #[tokio::test]
    async fn unsigned_delivery_omits_signature_headers() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let request = read_full_request(&mut socket).await;
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                .await
                .unwrap();

            request
        });

        let outcome = call_webhook(
            &default_webhook_client(),
            &WebhookDeliveryRequest {
                target_url: format!("http://{}/hook", addr),
                webhook_id: "w".to_string(),
                event_name: "e".to_string(),
                secret: None,
                payload_json: "{}".to_string(),
            },
        )
        .await;

        assert!(outcome.success);
        let raw = server.await.unwrap();
        assert!(!raw.to_lowercase().contains("x-twenty-webhook-signature"));
        assert!(!raw.to_lowercase().contains("x-twenty-webhook-timestamp"));
        assert!(!raw.to_lowercase().contains("x-twenty-webhook-nonce"));
    }

    #[tokio::test]
    async fn delivery_reports_non_success_status() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let _ = read_full_request(&mut socket).await;
            socket
                .write_all(b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n")
                .await
                .unwrap();
        });

        let outcome = call_webhook(
            &default_webhook_client(),
            &WebhookDeliveryRequest {
                target_url: format!("http://{}/hook", addr),
                webhook_id: "w".to_string(),
                event_name: "e".to_string(),
                secret: Some(generate_webhook_secret()),
                payload_json: "{}".to_string(),
            },
        )
        .await;

        server.await.unwrap();
        assert_eq!(outcome.status, Some(500));
        assert!(!outcome.success);
        assert_eq!(outcome.error, None);
    }

    #[tokio::test]
    async fn delivery_reports_connection_error() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let outcome = call_webhook(
            &default_webhook_client(),
            &WebhookDeliveryRequest {
                target_url: format!("http://{}/hook", addr),
                webhook_id: "w".to_string(),
                event_name: "e".to_string(),
                secret: None,
                payload_json: "{}".to_string(),
            },
        )
        .await;

        assert!(!outcome.success);
        assert_eq!(outcome.status, None);
        assert!(outcome.error.is_some());
    }
}
