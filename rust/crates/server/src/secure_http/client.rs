use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use reqwest::header::{HeaderName, HeaderValue};
use reqwest::redirect::Policy;
use reqwest::{Body, Method, Request, Response, Url};
use serde::Serialize;

use crate::secure_http::error::{
    blocked_connection_message, blocked_protocol_message, blocked_request_message,
    describe_outcome, find_blocked_message, redact_outbound_request_url, OutboundRequestContext,
    OutboundRequestOutcome, SecureHttpError, MAX_REDIRECTS_MESSAGE,
};
use crate::secure_http::ip_range::{parse_host_address, strip_ipv6_brackets};
use crate::secure_http::policy::GuardPolicy;
use crate::secure_http::resolver::{
    DnsCache, DnsLookup, GuardedResolver, SystemLookup, DEFAULT_DNS_CACHE_CAPACITY,
    DEFAULT_DNS_CACHE_TTL_MS,
};

pub const OUTBOUND_HTTP_DEFAULT_TIMEOUT_MS: u64 = 60_000;
pub const OUTBOUND_HTTP_DEFAULT_MAX_PAYLOAD_SIZE_BYTES: u64 = 50 * 1024 * 1024;
pub const MAX_REDIRECTS: usize = 5;
pub const HTTP_SCHEME: &str = "http";
pub const HTTPS_SCHEME: &str = "https";

#[derive(Debug, Clone)]
pub struct SecureHttpClientConfig {
    pub timeout: Duration,
    pub connect_timeout: Duration,
    pub max_redirects: usize,
    pub max_payload_bytes: u64,
    pub dns_cache_ttl: Duration,
    pub dns_cache_capacity: usize,
    pub respect_env_proxy: bool,
    pub pool_idle_per_host: usize,
}

impl Default for SecureHttpClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_millis(OUTBOUND_HTTP_DEFAULT_TIMEOUT_MS),
            connect_timeout: Duration::from_secs(10),
            max_redirects: MAX_REDIRECTS,
            max_payload_bytes: OUTBOUND_HTTP_DEFAULT_MAX_PAYLOAD_SIZE_BYTES,
            dns_cache_ttl: Duration::from_millis(DEFAULT_DNS_CACHE_TTL_MS),
            dns_cache_capacity: DEFAULT_DNS_CACHE_CAPACITY,
            respect_env_proxy: true,
            pool_idle_per_host: 32,
        }
    }
}

impl SecureHttpClientConfig {
    pub fn from_env() -> Self {
        let defaults = Self::default();
        Self {
            timeout: Duration::from_millis(crate::secure_http::policy::env_number(
                "OUTBOUND_HTTP_TIMEOUT_MS",
                defaults.timeout.as_millis() as u64,
            )),
            max_redirects: crate::secure_http::policy::env_number(
                "OUTBOUND_HTTP_MAX_REDIRECTS",
                defaults.max_redirects as u64,
            )
            .min(MAX_REDIRECTS as u64) as usize,
            max_payload_bytes: crate::secure_http::policy::env_number(
                "OUTBOUND_HTTP_MAX_PAYLOAD_SIZE_BYTES",
                defaults.max_payload_bytes,
            ),
            dns_cache_ttl: Duration::from_millis(crate::secure_http::policy::env_number(
                "OUTBOUND_HTTP_DNS_CACHE_TTL_MS",
                defaults.dns_cache_ttl.as_millis() as u64,
            )),
            dns_cache_capacity: (crate::secure_http::policy::env_number(
                "OUTBOUND_HTTP_DNS_CACHE_CAPACITY",
                defaults.dns_cache_capacity as u64,
            )
            .max(1)) as usize,
            respect_env_proxy: crate::secure_http::policy::env_flag(
                "OUTBOUND_HTTP_RESPECT_ENV_PROXY",
                true,
            ),
            ..defaults
        }
    }
}

#[derive(Debug, Clone)]
pub struct SecureHttpClient {
    inner: reqwest::Client,
    policy: Arc<GuardPolicy>,
    resolver: GuardedResolver,
    config: SecureHttpClientConfig,
}

impl SecureHttpClient {
    pub fn new(
        policy: GuardPolicy,
        config: SecureHttpClientConfig,
    ) -> Result<Self, SecureHttpError> {
        Self::with_lookup(policy, config, Arc::new(SystemLookup))
    }

    pub fn with_lookup(
        policy: GuardPolicy,
        config: SecureHttpClientConfig,
        lookup: Arc<dyn DnsLookup>,
    ) -> Result<Self, SecureHttpError> {
        let policy = Arc::new(policy);
        let cache = Arc::new(DnsCache::new(
            config.dns_cache_ttl,
            config.dns_cache_capacity,
        ));
        let resolver = GuardedResolver::new(policy.clone(), cache, lookup);
        let max_redirects = if policy.is_enforced() {
            config.max_redirects.min(MAX_REDIRECTS)
        } else {
            config.max_redirects
        };
        let mut builder = reqwest::Client::builder()
            .timeout(config.timeout)
            .connect_timeout(config.connect_timeout)
            .pool_max_idle_per_host(config.pool_idle_per_host)
            .dns_resolver2(resolver.clone())
            .redirect(redirect_policy(
                policy.clone(),
                resolver.cache().clone(),
                max_redirects,
            ));
        if !config.respect_env_proxy {
            builder = builder.no_proxy();
        }
        let inner = builder.build().map_err(SecureHttpError::Transport)?;
        Ok(Self {
            inner,
            policy,
            resolver,
            config,
        })
    }

    pub fn from_env() -> Result<Self, SecureHttpError> {
        Self::new(GuardPolicy::from_env(), SecureHttpClientConfig::from_env())
    }

    pub fn public_hosts_only() -> Result<Self, SecureHttpError> {
        Self::new(
            GuardPolicy::public_hosts_only(),
            SecureHttpClientConfig::from_env(),
        )
    }

    pub fn shared() -> Result<Arc<Self>, SecureHttpError> {
        static SHARED: OnceLock<Arc<SecureHttpClient>> = OnceLock::new();
        if let Some(existing) = SHARED.get() {
            return Ok(existing.clone());
        }
        let built = Arc::new(Self::from_env()?);
        Ok(Arc::clone(SHARED.get_or_init(|| built)))
    }

    pub fn policy(&self) -> &GuardPolicy {
        &self.policy
    }

    pub fn config(&self) -> &SecureHttpClientConfig {
        &self.config
    }

    pub fn dns_cache(&self) -> &Arc<DnsCache> {
        self.resolver.cache()
    }

    pub fn get(&self, url: &str) -> GuardedRequest<'_> {
        self.request(Method::GET, url)
    }

    pub fn post(&self, url: &str) -> GuardedRequest<'_> {
        self.request(Method::POST, url)
    }

    pub fn put(&self, url: &str) -> GuardedRequest<'_> {
        self.request(Method::PUT, url)
    }

    pub fn patch(&self, url: &str) -> GuardedRequest<'_> {
        self.request(Method::PATCH, url)
    }

    pub fn delete(&self, url: &str) -> GuardedRequest<'_> {
        self.request(Method::DELETE, url)
    }

    pub fn request(&self, method: Method, url: &str) -> GuardedRequest<'_> {
        GuardedRequest {
            client: self,
            builder: match Url::parse(url) {
                Ok(parsed) => match assert_allowed_protocol(&parsed) {
                    Ok(()) => Ok(self.inner.request(method, url)),
                    Err(error) => Err(error),
                },
                Err(_) => Err(SecureHttpError::InvalidUrl(url.to_string())),
            },
        }
    }

    pub async fn execute(&self, request: Request) -> Result<Response, SecureHttpError> {
        self.execute_with_context(request, None).await
    }

    pub async fn execute_with_context(
        &self,
        request: Request,
        context: Option<&OutboundRequestContext>,
    ) -> Result<Response, SecureHttpError> {
        let url = request.url().clone();
        let method = request.method().to_string();
        let attributes = build_request_attributes(&method, &url, context);
        if context.is_some() {
            tracing::info!("Outbound HTTP request: {}", join_attributes(&attributes));
        }
        let started_at = Instant::now();
        let result = self.dispatch(&url, request).await;
        if context.is_some() {
            let outcome = match &result {
                Ok(response) => describe_outcome(response.status().as_u16()),
                Err(error) => error.details(),
            };
            let line = join_attributes(&describe_attributes(
                &attributes,
                &outcome,
                started_at.elapsed().as_millis(),
            ));
            if outcome.outcome == OutboundRequestOutcome::Blocked {
                tracing::warn!("Outbound HTTP outcome: {line}");
            } else {
                tracing::info!("Outbound HTTP outcome: {line}");
            }
        }
        result
    }

    async fn dispatch(&self, url: &Url, request: Request) -> Result<Response, SecureHttpError> {
        self.preflight(url).await?;
        let response = self
            .inner
            .execute(request)
            .await
            .map_err(map_transport_error)?;
        self.enforce_payload_limit(&response)?;
        Ok(response)
    }

    async fn preflight(&self, url: &Url) -> Result<(), SecureHttpError> {
        assert_allowed_protocol(url)?;
        let host = url
            .host_str()
            .ok_or_else(|| SecureHttpError::InvalidUrl(redact_outbound_request_url(url)))?;
        self.resolver.resolve_validated(host).await?;
        Ok(())
    }

    fn enforce_payload_limit(&self, response: &Response) -> Result<(), SecureHttpError> {
        match response.content_length() {
            Some(declared) if declared > self.config.max_payload_bytes => {
                Err(SecureHttpError::PayloadTooLarge {
                    limit: self.config.max_payload_bytes,
                    declared,
                })
            }
            _ => Ok(()),
        }
    }

    pub async fn validated_host(&self, hostname_or_url: &str) -> Result<String, SecureHttpError> {
        let trimmed = hostname_or_url.trim();
        let hostname = Url::parse(trimmed)
            .ok()
            .and_then(|url| url.host_str().map(str::to_string))
            .unwrap_or_else(|| trimmed.to_string());
        if !self.policy.is_enforced() {
            return Ok(hostname);
        }
        let addresses = self.resolver.resolve_validated(&hostname).await?;
        addresses
            .first()
            .map(|address| address.ip().to_string())
            .ok_or_else(|| SecureHttpError::Lookup(format!("No addresses found for {hostname}.")))
    }
}

fn map_transport_error(error: reqwest::Error) -> SecureHttpError {
    match find_blocked_message(&error) {
        Some(message) => SecureHttpError::Blocked(message),
        None => SecureHttpError::Transport(error),
    }
}

fn assert_allowed_protocol(url: &Url) -> Result<(), SecureHttpError> {
    if url.scheme() == HTTP_SCHEME || url.scheme() == HTTPS_SCHEME {
        return Ok(());
    }
    Err(SecureHttpError::blocked(blocked_protocol_message(
        &format!("{}:", url.scheme()),
    )))
}

fn redirect_policy(policy: Arc<GuardPolicy>, cache: Arc<DnsCache>, max_redirects: usize) -> Policy {
    Policy::custom(move |attempt| {
        if attempt.previous().len() > max_redirects {
            return attempt.error(MAX_REDIRECTS_MESSAGE.to_string());
        }
        let target = attempt.url().clone();
        if target.scheme() != HTTP_SCHEME && target.scheme() != HTTPS_SCHEME {
            return attempt.error(blocked_protocol_message(&format!("{}:", target.scheme())));
        }
        let Some(host) = target.host_str().map(str::to_string) else {
            return attempt.error(blocked_request_message(&redact_outbound_request_url(
                &target,
            )));
        };
        if !policy.is_enforced() {
            return attempt.follow();
        }
        let bare = strip_ipv6_brackets(&host);
        if let Some(address) = parse_host_address(bare) {
            if policy.is_blocked_address(&host, &address.to_string()) {
                return attempt.error(blocked_request_message(&address.to_string()));
            }
            return attempt.follow();
        }
        if let Some(addresses) = cache.get(&host) {
            for address in &addresses {
                if policy
                    .check_address(&host, &address.ip().to_string())
                    .blocked()
                {
                    return attempt.error(blocked_connection_message(&address.ip().to_string()));
                }
            }
        }
        attempt.follow()
    })
}

fn build_request_attributes(
    method: &str,
    url: &Url,
    context: Option<&OutboundRequestContext>,
) -> Vec<(String, String)> {
    let mut attributes = vec![
        ("method".to_string(), method.to_string()),
        (
            "host".to_string(),
            url.host_str().unwrap_or_default().to_string(),
        ),
        ("url".to_string(), redact_outbound_request_url(url)),
    ];
    if let Some(context) = context {
        attributes.push(("workspace".to_string(), context.workspace_id.clone()));
        attributes.push(("source".to_string(), context.source.to_string()));
        for (key, value) in [
            ("user".to_string(), &context.user_id),
            ("userWorkspace".to_string(), &context.user_workspace_id),
            ("workspaceMember".to_string(), &context.workspace_member_id),
            ("workflowRun".to_string(), &context.workflow_run_id),
            ("workflowStep".to_string(), &context.workflow_step_id),
        ] {
            if let Some(value) = value {
                attributes.push((key.clone(), value.clone()));
            }
        }
    }
    attributes
}

fn describe_attributes(
    attributes: &[(String, String)],
    outcome: &crate::secure_http::error::OutboundRequestOutcomeDetails,
    duration_ms: u128,
) -> Vec<(String, String)> {
    let mut described = vec![("outcome".to_string(), outcome.outcome.to_string())];
    if let Some(status) = outcome.status {
        described.push(("status".to_string(), status.to_string()));
    }
    described.push(("durationMs".to_string(), duration_ms.to_string()));
    if let Some(reason) = &outcome.reason {
        described.push(("reason".to_string(), reason.clone()));
    }
    described.extend(attributes.iter().cloned());
    described
}

fn join_attributes(attributes: &[(String, String)]) -> String {
    attributes
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<String>>()
        .join(" ")
}

pub struct GuardedRequest<'a> {
    client: &'a SecureHttpClient,
    builder: Result<reqwest::RequestBuilder, SecureHttpError>,
}

impl<'a> GuardedRequest<'a> {
    pub fn header(self, name: &str, value: &str) -> Self {
        let client = self.client;
        let builder = match self.builder {
            Err(error) => Err(error),
            Ok(builder) => match (HeaderName::try_from(name), HeaderValue::try_from(value)) {
                (Ok(parsed_name), Ok(parsed_value)) => {
                    Ok(builder.header(parsed_name, parsed_value))
                }
                (Err(_), _) | (_, Err(_)) => {
                    Err(SecureHttpError::InvalidHeader(format!("{name}: {value}")))
                }
            },
        };
        Self { client, builder }
    }

    pub fn typed_header(self, name: HeaderName, value: HeaderValue) -> Self {
        self.map(move |builder| builder.header(name, value))
    }

    pub fn headers(self, headers: reqwest::header::HeaderMap) -> Self {
        self.map(move |builder| builder.headers(headers))
    }

    pub fn bearer_auth(self, token: &str) -> Self {
        self.map(move |builder| builder.bearer_auth(token))
    }

    pub fn query<T: Serialize + ?Sized>(self, query: &T) -> Self {
        self.map(move |builder| builder.query(query))
    }

    pub fn json<T: Serialize + ?Sized>(self, value: &T) -> Self {
        self.map(move |builder| builder.json(value))
    }

    pub fn form<T: Serialize + ?Sized>(self, form: &T) -> Self {
        self.map(move |builder| builder.form(form))
    }

    pub fn body(self, body: impl Into<Body>) -> Self {
        self.map(move |builder| builder.body(body))
    }

    pub fn timeout(self, timeout: Duration) -> Self {
        self.map(move |builder| builder.timeout(timeout))
    }

    pub fn basic_auth(self, user: &str, password: &str) -> Self {
        self.map(move |builder| builder.basic_auth(user, Some(password)))
    }

    fn map<F>(self, transform: F) -> Self
    where
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        Self {
            client: self.client,
            builder: match self.builder {
                Ok(builder) => Ok(transform(builder)),
                Err(error) => Err(error),
            },
        }
    }

    pub async fn send(self) -> Result<Response, SecureHttpError> {
        let builder = self.builder?;
        let request = builder.build().map_err(SecureHttpError::Transport)?;
        self.client.execute(request).await
    }

    pub async fn send_with_context(
        self,
        context: &OutboundRequestContext,
    ) -> Result<Response, SecureHttpError> {
        let builder = self.builder?;
        let request = builder.build().map_err(SecureHttpError::Transport)?;
        self.client
            .execute_with_context(request, Some(context))
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::net::{IpAddr, SocketAddr};
    use std::sync::{Arc, Mutex};

    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::*;
    use crate::secure_http::error::OutboundRequestSource;
    use crate::secure_http::resolver::LookupFuture;

    #[derive(Debug, Default)]
    struct StubLookup {
        answers: Mutex<HashMap<String, Vec<IpAddr>>>,
        calls: Mutex<Vec<String>>,
    }

    impl StubLookup {
        fn set(&self, host: &str, address: &str) {
            let parsed = address
                .parse::<IpAddr>()
                .unwrap_or_else(|_| panic!("{address} parses"));
            if let Ok(mut answers) = self.answers.lock() {
                answers.insert(host.to_lowercase(), vec![parsed]);
            }
        }

        fn new(entries: &[(&str, &str)]) -> Arc<Self> {
            let mut answers: HashMap<String, Vec<IpAddr>> = HashMap::new();
            for (host, address) in entries {
                let parsed = address
                    .parse::<IpAddr>()
                    .unwrap_or_else(|_| panic!("{address} parses"));
                answers.insert((*host).to_lowercase(), vec![parsed]);
            }
            Arc::new(Self {
                answers: Mutex::new(answers),
                calls: Mutex::new(Vec::new()),
            })
        }

        fn calls(&self) -> Vec<String> {
            self.calls
                .lock()
                .map(|calls| calls.clone())
                .unwrap_or_default()
        }
    }

    impl DnsLookup for StubLookup {
        fn lookup(&self, host: String) -> LookupFuture {
            if let Ok(mut calls) = self.calls.lock() {
                calls.push(host.clone());
            }
            let answer = self
                .answers
                .lock()
                .ok()
                .and_then(|answers| {
                    answers.get(&host.to_lowercase()).map(|addresses| {
                        addresses.iter().map(|ip| SocketAddr::new(*ip, 0)).collect()
                    })
                })
                .unwrap_or_default();
            Box::pin(async move { Ok(answer) })
        }
    }

    #[derive(Clone, Debug)]
    enum StubMode {
        Ok,
        Redirect(String),
        Declared(usize),
    }

    #[derive(Debug, Clone)]
    struct Stub {
        address: SocketAddr,
        seen: Arc<Mutex<Vec<String>>>,
    }

    impl Stub {
        fn requests(&self) -> Vec<String> {
            self.seen
                .lock()
                .map(|seen| seen.clone())
                .unwrap_or_default()
        }
    }

    async fn spawn_stub(mode: StubMode) -> Stub {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("binding the stub server");
        let address = listener.local_addr().expect("stub address");
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                let mut buffer = Vec::new();
                let mut chunk = [0u8; 1024];
                for _ in 0..8 {
                    let Ok(read) = socket.read(&mut chunk).await else {
                        break;
                    };
                    if read == 0 {
                        break;
                    }
                    buffer.extend_from_slice(&chunk[..read]);
                    if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                let text = String::from_utf8_lossy(&buffer).to_string();
                let first_line = text.lines().next().unwrap_or_default().to_string();
                let host_line = text
                    .lines()
                    .find(|line| line.to_ascii_lowercase().starts_with("host:"))
                    .unwrap_or_default()
                    .to_string();
                if let Ok(mut guard) = recorded.lock() {
                    guard.push(format!("{first_line} {host_line}"));
                }
                let response = match &mode {
                    StubMode::Redirect(target) => {
                        let target = target.replace("{port}", &address.port().to_string());
                        format!(
                            "HTTP/1.1 302 Found
location: {target}
content-length: 0
connection: close

"
                        )
                    }
                    StubMode::Declared(length) => format!(
                        "HTTP/1.1 200 OK\r\ncontent-length: {length}\r\nconnection: close\r\n\r\n"
                    ),
                    StubMode::Ok => {
                        "HTTP/1.1 200 OK\r\ncontent-length: 2\r\nconnection: close\r\n\r\nok"
                            .to_string()
                    }
                };
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
            }
        });
        Stub { address, seen }
    }

    fn test_config() -> SecureHttpClientConfig {
        SecureHttpClientConfig {
            timeout: Duration::from_secs(5),
            connect_timeout: Duration::from_secs(2),
            respect_env_proxy: false,
            ..SecureHttpClientConfig::default()
        }
    }

    fn guarded(policy: GuardPolicy, lookup: Arc<StubLookup>) -> SecureHttpClient {
        SecureHttpClient::with_lookup(policy, test_config(), lookup).expect("building the guard")
    }

    fn assert_blocked_phrase(error: &SecureHttpError) -> String {
        let message = error.message();
        assert!(
            message.contains("internal IP address") && message.contains("is not allowed"),
            "message {message} keeps the upstream classification"
        );
        assert!(error.is_blocked(), "{message}");
        assert_eq!(error.outcome(), OutboundRequestOutcome::Blocked);
        message
    }

    #[tokio::test]
    async fn literal_internal_addresses_are_refused_without_dns() {
        let lookup = StubLookup::new(&[]);
        let client = guarded(GuardPolicy::public_hosts_only(), lookup.clone());
        for url in [
            "http://127.0.0.1:9/private",
            "http://169.254.169.254/latest/meta-data/",
            "http://10.0.0.7/",
            "http://[::1]:80/private",
            "http://[fd00::1]/private",
            "http://100.64.0.1/private",
            "http://0.0.0.0/private",
            "http://192.168.1.1/private",
            "http://172.16.0.1/private",
            "http://192.0.2.1/private",
            "http://198.18.0.1/private",
            "http://203.0.113.1/private",
            "http://224.0.0.1/private",
            "http://240.0.0.1/private",
            "http://192.0.0.1/private",
            "http://198.51.100.1/private",
            "http://[fe80::1]/private",
            "http://[::ffff:169.254.169.254]/private",
        ] {
            let error = client.get(url).send().await.expect_err("{url} is refused");
            assert_blocked_phrase(&error);
        }
        assert!(lookup.calls().is_empty(), "no DNS for literal hosts");
    }

    #[tokio::test]
    async fn hostname_resolving_to_internal_address_is_blocked() {
        let lookup = StubLookup::new(&[("rebind.example", "127.0.0.1")]);
        let client = guarded(GuardPolicy::public_hosts_only(), lookup.clone());
        let error = client
            .get("http://rebind.example/labels/latest/meta-data")
            .send()
            .await
            .expect_err("rebind blocked");
        assert_eq!(
            assert_blocked_phrase(&error),
            "Connection to internal IP address 127.0.0.1 is not allowed."
        );
        assert_eq!(lookup.calls(), vec!["rebind.example".to_string()]);
    }

    #[tokio::test]
    async fn the_system_resolver_answer_is_validated_before_dialing() {
        let client = SecureHttpClient::with_lookup(
            GuardPolicy::public_hosts_only(),
            test_config(),
            Arc::new(SystemLookup),
        )
        .expect("building the guard");
        let error = client
            .get("http://localhost:9/private")
            .send()
            .await
            .expect_err("localhost resolves into the blocked range");
        let message = error.message();
        assert!(
            message.starts_with("Connection to internal IP address ")
                && message.ends_with(" is not allowed."),
            "{message}"
        );
        assert!(
            message.contains("127.0.0.1") || message.contains("::1"),
            "{message}"
        );
        assert_eq!(error.outcome(), OutboundRequestOutcome::Blocked);
        assert!(
            client.dns_cache().is_empty(),
            "a refused answer is never cached"
        );
        assert!(matches!(
            client
                .get("ftp://localhost:21/private")
                .send()
                .await
                .expect_err("ftp is refused"),
            SecureHttpError::Blocked(_)
        ));
    }

    #[tokio::test]
    async fn validated_addresses_are_pinned_and_the_host_header_is_preserved() {
        let stub = spawn_stub(StubMode::Ok).await;
        let lookup = StubLookup::new(&[("pinned.example", "127.0.0.1")]);
        let client = guarded(GuardPolicy::new(["pinned.example"]), lookup);
        let response = client
            .get(&format!(
                "http://pinned.example:{}/avatar.png",
                stub.address.port()
            ))
            .send()
            .await
            .expect("pinned request reaches the stub");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert_eq!(response.text().await.expect("body"), "ok");
        let seen = stub.requests();
        assert_eq!(seen.len(), 1);
        assert!(
            seen[0].contains("GET /avatar.png HTTP/1.1 host: pinned.example"),
            "{}",
            seen[0]
        );
    }

    #[tokio::test]
    async fn redirect_to_internal_literal_is_blocked() {
        let stub = spawn_stub(StubMode::Redirect(
            "http://127.0.0.1:{port}/metadata".to_string(),
        ))
        .await;
        let lookup = StubLookup::new(&[("hop.example", "127.0.0.1")]);
        let client = guarded(GuardPolicy::new(["hop.example"]), lookup);
        let error = client
            .get(&format!("http://hop.example:{}/start", stub.address.port()))
            .send()
            .await
            .expect_err("redirect target blocked");
        assert_blocked_phrase(&error);
    }

    #[tokio::test]
    async fn redirect_to_hostname_resolving_internally_is_blocked() {
        let stub = spawn_stub(StubMode::Redirect(
            "http://evil.example/metadata".to_string(),
        ))
        .await;
        let lookup = StubLookup::new(&[
            ("hop.example", "127.0.0.1"),
            ("evil.example", "169.254.169.254"),
        ]);
        let client = guarded(GuardPolicy::new(["hop.example"]), lookup);
        let error = client
            .get(&format!("http://hop.example:{}/start", stub.address.port()))
            .send()
            .await
            .expect_err("redirect target blocked");
        assert_eq!(
            assert_blocked_phrase(&error),
            "Connection to internal IP address 169.254.169.254 is not allowed."
        );
    }

    #[tokio::test]
    async fn redirect_hops_are_capped_at_five() {
        let stub = spawn_stub(StubMode::Redirect(
            "http://hop.example:{port}/again".to_string(),
        ))
        .await;
        let lookup = StubLookup::new(&[("hop.example", "127.0.0.1")]);
        let client = guarded(GuardPolicy::new(["hop.example"]), lookup);
        let error = client
            .get(&format!("http://hop.example:{}/start", stub.address.port()))
            .send()
            .await
            .expect_err("redirect loop stops");
        assert!(
            error.message().contains(MAX_REDIRECTS_MESSAGE),
            "{}",
            error.message()
        );
        assert_eq!(error.outcome(), OutboundRequestOutcome::OtherError);
        assert_eq!(stub.requests().len(), MAX_REDIRECTS + 1);
    }

    #[tokio::test]
    async fn the_socket_dials_the_validated_answer_not_system_dns() {
        let stub = spawn_stub(StubMode::Ok).await;
        let lookup = StubLookup::new(&[("unresolvable.invalid", "127.0.0.1")]);
        let client = guarded(GuardPolicy::new(["unresolvable.invalid"]), lookup);
        let response = client
            .get(&format!(
                "http://unresolvable.invalid:{}/pin",
                stub.address.port()
            ))
            .send()
            .await
            .expect("validated address is dialed");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let seen = stub.requests();
        assert_eq!(seen.len(), 1);
        assert!(seen[0].contains("host: unresolvable.invalid"));
    }

    #[tokio::test]
    async fn a_changed_dns_answer_cannot_retarget_a_pinned_connection() {
        let stub = spawn_stub(StubMode::Ok).await;
        let lookup = StubLookup::new(&[("drift.example", "127.0.0.1")]);
        let client = guarded(GuardPolicy::new(["drift.example"]), lookup.clone());
        let url = format!("http://drift.example:{}/first", stub.address.port());
        client
            .get(&url)
            .send()
            .await
            .expect("first hop")
            .text()
            .await
            .expect("first body");
        lookup.set("drift.example", "169.254.169.254");
        let response = client
            .get(&url)
            .send()
            .await
            .expect("pinned hop stays on the validated address");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert_eq!(stub.requests().len(), 2);
        let expired = SecureHttpClientConfig {
            dns_cache_ttl: Duration::ZERO,
            ..test_config()
        };
        let fresh = SecureHttpClient::with_lookup(
            GuardPolicy::new(["drift.example"]),
            expired,
            lookup.clone(),
        )
        .expect("building the guard");
        let error = fresh
            .get(&url)
            .send()
            .await
            .expect_err("refreshed answer is revalidated");
        assert_blocked_phrase(&error);
    }

    #[tokio::test]
    async fn non_http_protocols_are_blocked() {
        let lookup = StubLookup::new(&[]);
        let client = guarded(GuardPolicy::public_hosts_only(), lookup);
        for url in ["file:///etc/passwd", "gopher://127.0.0.1:70/x", "http://"] {
            let error = client.get(url).send().await.expect_err("{url} refused");
            let message = error.message();
            assert!(
                message.contains("is not allowed. Only HTTP and HTTPS are permitted.")
                    || message.contains("Invalid URL"),
                "{message}"
            );
        }
    }

    #[tokio::test]
    async fn public_targets_pass_validation() {
        let lookup = StubLookup::new(&[("api.example", "93.184.216.34")]);
        let client = guarded(GuardPolicy::public_hosts_only(), lookup);
        assert_eq!(
            client
                .validated_host("api.example")
                .await
                .expect("public hostname allowed"),
            "93.184.216.34"
        );
        assert_eq!(
            client
                .validated_host("http://8.8.8.8/v1")
                .await
                .expect("public literal allowed"),
            "8.8.8.8"
        );
        assert_eq!(
            client
                .validated_host("  https://API.Example/x  ")
                .await
                .expect("hostname normalized"),
            "93.184.216.34"
        );
    }

    #[tokio::test]
    async fn allowlisted_host_still_refuses_link_local_answers() {
        let lookup = StubLookup::new(&[("internal.example", "169.254.169.254")]);
        let client = guarded(GuardPolicy::new(["internal.example"]), lookup);
        let error = client
            .get("http://internal.example/x")
            .send()
            .await
            .expect_err("metadata answer blocked");
        assert_blocked_phrase(&error);
    }

    #[tokio::test]
    async fn answers_are_cached_across_requests() {
        let stub = spawn_stub(StubMode::Ok).await;
        let lookup = StubLookup::new(&[("cache.example", "127.0.0.1")]);
        let client = guarded(GuardPolicy::new(["cache.example"]), lookup.clone());
        let url = format!("http://cache.example:{}/a", stub.address.port());
        client
            .get(&url)
            .send()
            .await
            .expect("first request")
            .text()
            .await
            .expect("first body");
        client
            .get(&url)
            .send()
            .await
            .expect("second request")
            .text()
            .await
            .expect("second body");
        assert_eq!(lookup.calls(), vec!["cache.example".to_string()]);
        assert_eq!(stub.requests().len(), 2);
    }

    #[tokio::test]
    async fn declared_payload_over_the_limit_is_refused() {
        let stub = spawn_stub(StubMode::Declared(20_000)).await;
        let lookup = StubLookup::new(&[("big.example", "127.0.0.1")]);
        let config = SecureHttpClientConfig {
            max_payload_bytes: 1024,
            ..test_config()
        };
        let client =
            SecureHttpClient::with_lookup(GuardPolicy::new(["big.example"]), config, lookup)
                .expect("building the guard");
        let error = client
            .get(&format!("http://big.example:{}/blob", stub.address.port()))
            .send()
            .await
            .expect_err("oversized body refused");
        assert!(matches!(
            error,
            SecureHttpError::PayloadTooLarge { limit: 1024, .. }
        ));
    }

    #[tokio::test]
    async fn disabled_safe_mode_reaches_private_addresses() {
        let stub = spawn_stub(StubMode::Ok).await;
        let lookup = StubLookup::new(&[]);
        let client = guarded(GuardPolicy::allow_all(), lookup.clone());
        let response = client
            .get(&format!("http://127.0.0.1:{}/open", stub.address.port()))
            .send()
            .await
            .expect("guard disabled");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert!(lookup.calls().is_empty());
    }

    #[tokio::test]
    async fn context_logging_reports_the_blocked_outcome() {
        let lookup = StubLookup::new(&[("secret.example", "10.1.2.3")]);
        let client = guarded(GuardPolicy::public_hosts_only(), lookup);
        let context = OutboundRequestContext::new("workspace-1", OutboundRequestSource::Webhook);
        let error = client
            .get("https://secret.example/path?token=leaked")
            .send_with_context(&context)
            .await
            .expect_err("blocked with context");
        let details = error.details();
        assert_eq!(details.outcome, OutboundRequestOutcome::Blocked);
        assert_eq!(
            details.reason.as_deref(),
            Some("Connection to internal IP address 10.1.2.3 is not allowed.")
        );
        let redacted = redact_outbound_request_url(
            &Url::parse("https://secret.example/path?token=leaked").expect("parse url"),
        );
        assert_eq!(redacted, "https://secret.example/path");
    }

    #[tokio::test]
    async fn request_builders_forward_through_the_guard() {
        let stub = spawn_stub(StubMode::Ok).await;
        let lookup = StubLookup::new(&[("post.example", "127.0.0.1")]);
        let client = guarded(GuardPolicy::new(["post.example"]), lookup);
        let response = client
            .post(&format!("http://post.example:{}/hook", stub.address.port()))
            .header("x-nudge-test", "1")
            .json(&serde_json::json!({ "event": "created" }))
            .send()
            .await
            .expect("post allowed");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let seen = stub.requests();
        assert_eq!(seen.len(), 1);
        assert!(seen[0].contains("POST /hook"));
    }

    #[tokio::test]
    async fn execute_validates_prepared_requests() {
        let lookup = StubLookup::new(&[("prepared.example", "127.0.0.1")]);
        let client = guarded(GuardPolicy::public_hosts_only(), lookup);
        let request = client
            .inner
            .get("http://prepared.example/x")
            .build()
            .expect("building the request");
        let error = client
            .execute(request)
            .await
            .expect_err("prepared request blocked");
        assert_blocked_phrase(&error);
    }

    #[test]
    fn the_environment_allowlist_parses_comma_separated_entries() {
        let policy = GuardPolicy::new(crate::secure_http::policy::split_allowed_internal_hosts(
            " https://internal.example:8443 , OTHER.Example ",
        ));
        assert!(policy.is_allowed_internal_host("internal.example"));
        assert!(policy.is_allowed_internal_host("other.example"));
        assert!(!policy.is_allowed_internal_host("third.example"));
    }
}
