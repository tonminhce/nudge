use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use reqwest::dns::{Addrs, Name, Resolve, Resolving};

use crate::secure_http::error::{
    blocked_connection_message, blocked_request_message, unvalidatable_address_message,
    SecureHttpError,
};
use crate::secure_http::ip_range::{parse_host_address, strip_ipv6_brackets, AddressCheck};
use crate::secure_http::policy::GuardPolicy;

pub const DEFAULT_DNS_CACHE_TTL_MS: u64 = 5_000;
pub const DEFAULT_DNS_CACHE_CAPACITY: usize = 4_096;

pub type LookupFuture = Pin<Box<dyn Future<Output = std::io::Result<Vec<SocketAddr>>> + Send>>;

pub trait DnsLookup: Send + Sync + std::fmt::Debug {
    fn lookup(&self, host: String) -> LookupFuture;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemLookup;

impl DnsLookup for SystemLookup {
    fn lookup(&self, host: String) -> LookupFuture {
        Box::pin(async move {
            tokio::net::lookup_host((host, 0))
                .await
                .map(|resolved| resolved.collect())
        })
    }
}

#[derive(Debug, Clone)]
struct CachedLookup {
    addresses: Vec<SocketAddr>,
    inserted_at: Instant,
}

#[derive(Debug)]
pub struct DnsCache {
    entries: Mutex<HashMap<String, CachedLookup>>,
    ttl: Duration,
    capacity: usize,
}

impl DnsCache {
    pub fn new(ttl: Duration, capacity: usize) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            ttl,
            capacity: capacity.max(1),
        }
    }

    pub fn get(&self, host: &str) -> Option<Vec<SocketAddr>> {
        let key = host.to_ascii_lowercase();
        let mut guard = self.entries.lock().ok()?;
        if let Some(entry) = guard.get(&key) {
            if entry.inserted_at.elapsed() < self.ttl {
                return Some(entry.addresses.clone());
            }
            guard.remove(&key);
        }
        None
    }

    pub fn insert(&self, host: &str, addresses: Vec<SocketAddr>) {
        let Ok(mut guard) = self.entries.lock() else {
            return;
        };
        if guard.len() >= self.capacity {
            let oldest = guard
                .iter()
                .min_by_key(|(_, entry)| entry.inserted_at)
                .map(|(key, _)| key.clone());
            if let Some(key) = oldest {
                guard.remove(&key);
            }
        }
        guard.insert(
            host.to_ascii_lowercase(),
            CachedLookup {
                addresses,
                inserted_at: Instant::now(),
            },
        );
    }

    pub fn len(&self) -> usize {
        self.entries
            .lock()
            .map(|guard| guard.len())
            .unwrap_or_default()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&self) {
        if let Ok(mut guard) = self.entries.lock() {
            guard.clear();
        }
    }
}

#[derive(Debug, Clone)]
pub struct GuardedResolver {
    policy: Arc<GuardPolicy>,
    cache: Arc<DnsCache>,
    lookup: Arc<dyn DnsLookup>,
}

impl GuardedResolver {
    pub fn new(policy: Arc<GuardPolicy>, cache: Arc<DnsCache>, lookup: Arc<dyn DnsLookup>) -> Self {
        Self {
            policy,
            cache,
            lookup,
        }
    }

    pub fn policy(&self) -> &GuardPolicy {
        &self.policy
    }

    pub fn cache(&self) -> &Arc<DnsCache> {
        &self.cache
    }

    pub async fn resolve_validated(&self, host: &str) -> Result<Vec<SocketAddr>, SecureHttpError> {
        if let Some(address) = parse_host_address(host) {
            if self.policy.is_enforced() {
                self.validate(host, host)?;
            }
            return Ok(vec![SocketAddr::new(address, 0)]);
        }
        if let Some(addresses) = self.cache.get(host) {
            if self.policy.is_enforced() {
                self.validate_all(host, &addresses)?;
            }
            return Ok(addresses);
        }
        let addresses = self.resolve_raw(host).await?;
        if addresses.is_empty() {
            return Err(SecureHttpError::Lookup(format!(
                "No addresses found for {host}."
            )));
        }
        if self.policy.is_enforced() {
            self.validate_all(host, &addresses)?;
        }
        self.cache.insert(host, addresses.clone());
        Ok(addresses)
    }

    async fn resolve_raw(&self, host: &str) -> Result<Vec<SocketAddr>, SecureHttpError> {
        let owned = host.to_string();
        self.lookup
            .lookup(owned)
            .await
            .map_err(|error| SecureHttpError::Lookup(format!("Failed to lookup {host}: {error}")))
    }

    fn validate_all(&self, host: &str, addresses: &[SocketAddr]) -> Result<(), SecureHttpError> {
        for address in addresses {
            self.validate(host, &address.ip().to_string())?;
        }
        Ok(())
    }

    fn validate(&self, host: &str, label: &str) -> Result<(), SecureHttpError> {
        let host_is_literal = parse_host_address(strip_ipv6_brackets(host)).is_some();
        match self.policy.check_address(host, label) {
            AddressCheck::Allowed => Ok(()),
            AddressCheck::Blocked if host_is_literal => {
                Err(SecureHttpError::blocked(blocked_request_message(host)))
            }
            AddressCheck::Blocked => {
                Err(SecureHttpError::blocked(blocked_connection_message(label)))
            }
            AddressCheck::Unvalidatable => Err(SecureHttpError::blocked(
                unvalidatable_address_message(label),
            )),
        }
    }
}

impl Resolve for GuardedResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let resolver = self.clone();
        let host = name.as_str().to_string();
        Box::pin(async move {
            let addresses = resolver.resolve_validated(&host).await?;
            let iterator: Addrs = Box::new(addresses.into_iter());
            Ok(iterator)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    use super::*;

    #[derive(Debug, Default)]
    struct StubLookup {
        answers: Mutex<HashMap<String, Vec<SocketAddr>>>,
        calls: Mutex<Vec<String>>,
    }

    impl StubLookup {
        fn with(answers: &[(&str, &str)]) -> Arc<Self> {
            let mut map = HashMap::new();
            for (host, address) in answers {
                let ip: IpAddr = address.parse().expect("stub address");
                map.entry((*host).to_string())
                    .or_insert_with(Vec::new)
                    .push(SocketAddr::new(ip, 0));
            }
            Arc::new(Self {
                answers: Mutex::new(map),
                calls: Mutex::new(Vec::new()),
            })
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
                .and_then(|answers| answers.get(&host).cloned())
                .unwrap_or_default();
            Box::pin(async move { Ok(answer) })
        }
    }

    fn calls(stub: &Arc<StubLookup>) -> Vec<String> {
        stub.calls
            .lock()
            .map(|calls| calls.clone())
            .unwrap_or_default()
    }

    fn resolver(policy: GuardPolicy, stub: Arc<StubLookup>) -> GuardedResolver {
        GuardedResolver::new(
            Arc::new(policy),
            Arc::new(DnsCache::new(Duration::from_secs(30), 64)),
            stub,
        )
    }

    #[tokio::test]
    async fn literal_ip_hosts_are_validated_without_dns() {
        let stub = StubLookup::with(&[]);
        let guarded = resolver(GuardPolicy::public_hosts_only(), stub.clone());
        let error = guarded
            .resolve_validated("169.254.169.254")
            .await
            .expect_err("metadata address blocked");
        assert_eq!(
            error.message(),
            "Request to internal IP address 169.254.169.254 is not allowed."
        );
        assert!(calls(&stub).is_empty());
        let loopback6 = IpAddr::V6(Ipv6Addr::LOCALHOST);
        let error = guarded
            .resolve_validated(&format!("[{loopback6}]"))
            .await
            .expect_err("loopback blocked");
        assert!(error.to_string().contains("internal IP address"));
        assert_eq!(
            guarded
                .resolve_validated("8.8.8.8")
                .await
                .expect("public literal allowed")
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn rebinding_answers_are_blocked_before_connect() {
        let stub = StubLookup::with(&[("rebind.example", "127.0.0.1")]);
        let guarded = resolver(GuardPolicy::public_hosts_only(), stub.clone());
        let error = guarded
            .resolve_validated("rebind.example")
            .await
            .expect_err("rebind blocked");
        assert_eq!(
            error.message(),
            "Connection to internal IP address 127.0.0.1 is not allowed."
        );
        assert_eq!(calls(&stub).len(), 1);
    }

    #[tokio::test]
    async fn every_answer_in_a_multi_record_response_is_checked() {
        let stub = StubLookup::with(&[("split.example", "93.184.216.34")]);
        let shared = {
            let mut guard = stub.as_ref().answers.lock().expect("stub answers");
            guard
                .get_mut("split.example")
                .expect("entry")
                .push(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3)), 0));
            drop(guard);
            stub.clone()
        };
        let guarded = resolver(GuardPolicy::public_hosts_only(), shared);
        let error = guarded
            .resolve_validated("split.example")
            .await
            .expect_err("internal record blocks the whole answer");
        assert!(error
            .to_string()
            .contains("Connection to internal IP address 10.1.2.3 is not allowed"));
        assert_eq!(calls(&stub).len(), 1);
    }

    #[tokio::test]
    async fn answers_are_cached_and_reused_within_the_ttl() {
        let stub = StubLookup::with(&[("api.example", "93.184.216.34")]);
        let guarded = resolver(GuardPolicy::public_hosts_only(), stub.clone());
        guarded
            .resolve_validated("api.example")
            .await
            .expect("first lookup");
        guarded
            .resolve_validated("api.example")
            .await
            .expect("cached lookup");
        assert_eq!(calls(&stub).len(), 1);
        assert_eq!(guarded.cache().len(), 1);
        let short_lived = GuardedResolver::new(
            Arc::new(GuardPolicy::public_hosts_only()),
            Arc::new(DnsCache::new(Duration::ZERO, 64)),
            stub.clone(),
        );
        short_lived
            .resolve_validated("api.example")
            .await
            .expect("expired entry refetched");
        assert_eq!(calls(&stub).len(), 2);
    }

    #[tokio::test]
    async fn allowlisted_hosts_keep_link_local_closed() {
        let stub = StubLookup::with(&[
            ("internal.example", "10.0.0.5"),
            ("meta.example", "169.254.169.254"),
        ]);
        let guarded = resolver(
            GuardPolicy::new(["internal.example", "meta.example"]),
            stub.clone(),
        );
        assert_eq!(
            guarded
                .resolve_validated("internal.example")
                .await
                .expect("allowlisted private host")
                .len(),
            1
        );
        let error = guarded
            .resolve_validated("meta.example")
            .await
            .expect_err("link-local stays closed");
        assert!(error
            .to_string()
            .contains("Connection to internal IP address 169.254.169.254 is not allowed"));
    }

    #[tokio::test]
    async fn unparseable_answers_fail_closed() {
        let stub = StubLookup::with(&[]);
        let guarded = resolver(GuardPolicy::public_hosts_only(), stub);
        let error = guarded
            .resolve_validated("empty.example")
            .await
            .expect_err("no records");
        assert!(error.to_string().contains("No addresses found"));
    }

    #[test]
    fn cache_evicts_the_oldest_entry_at_capacity() {
        let cache = DnsCache::new(Duration::from_secs(60), 2);
        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        cache.insert("a.example", vec![address]);
        cache.insert("b.example", vec![address]);
        cache.insert("c.example", vec![address]);
        assert_eq!(cache.len(), 2);
        assert!(cache.get("c.example").is_some());
    }

    #[tokio::test]
    async fn unenforced_policy_still_returns_addresses() {
        let stub = StubLookup::with(&[("any.example", "127.0.0.1")]);
        let guarded = resolver(GuardPolicy::allow_all(), stub);
        let addresses = guarded
            .resolve_validated("any.example")
            .await
            .expect("guard disabled");
        assert_eq!(addresses[0].ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
    }
}
