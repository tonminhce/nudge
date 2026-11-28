use reqwest::Url;

use crate::secure_http::ip_range::{self, AddressCheck};

pub const ALLOW_ALL_INTERNAL_HOSTS: &str = "*";

pub fn normalize_allowed_internal_host(entry: &str) -> String {
    let trimmed = entry.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let candidate = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("http://{trimmed}")
    };
    match Url::parse(&candidate) {
        Ok(url) => url
            .host_str()
            .map(ip_range::strip_ipv6_brackets)
            .unwrap_or_default()
            .to_lowercase(),
        Err(_) => trimmed.to_lowercase(),
    }
}

pub fn normalize_host_for_match(host: &str) -> String {
    ip_range::strip_ipv6_brackets(host.trim()).to_lowercase()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardPolicy {
    allowed_internal_hosts: Vec<String>,
    enforced: bool,
}

impl GuardPolicy {
    pub fn new<I, S>(entries: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let allowed_internal_hosts: Vec<String> = entries
            .into_iter()
            .map(|entry| normalize_allowed_internal_host(entry.as_ref()))
            .filter(|entry| !entry.is_empty())
            .collect();
        let allow_all = allowed_internal_hosts
            .iter()
            .any(|entry| entry == ALLOW_ALL_INTERNAL_HOSTS);
        Self {
            allowed_internal_hosts,
            enforced: !allow_all,
        }
    }

    pub fn public_hosts_only() -> Self {
        Self {
            allowed_internal_hosts: Vec::new(),
            enforced: true,
        }
    }

    pub fn allow_all() -> Self {
        Self {
            allowed_internal_hosts: vec![ALLOW_ALL_INTERNAL_HOSTS.to_string()],
            enforced: false,
        }
    }

    pub fn from_env() -> Self {
        let safe_mode = env_flag("OUTBOUND_HTTP_SAFE_MODE_ENABLED", true);
        if !safe_mode {
            return Self::allow_all();
        }
        Self::new(env_list("OUTBOUND_HTTP_ALLOWED_INTERNAL_HOSTS"))
    }

    pub fn is_enforced(&self) -> bool {
        self.enforced
    }

    pub fn allowed_internal_hosts(&self) -> &[String] {
        &self.allowed_internal_hosts
    }

    pub fn is_allowed_internal_host(&self, host: &str) -> bool {
        if !self.enforced {
            return true;
        }
        let normalized = normalize_host_for_match(host);
        normalized != "*"
            && self
                .allowed_internal_hosts
                .iter()
                .any(|entry| entry == &normalized)
    }

    pub fn restricts_to_link_local(&self, host: &str) -> bool {
        self.is_allowed_internal_host(host)
    }

    pub fn check_address(&self, host: &str, address: &str) -> AddressCheck {
        if !self.enforced {
            return AddressCheck::Allowed;
        }
        ip_range::check_address(address, self.restricts_to_link_local(host))
    }

    pub fn is_blocked_address(&self, host: &str, address: &str) -> bool {
        self.check_address(host, address).blocked()
    }

    pub fn is_blocked_hostname(&self, host: &str) -> bool {
        if !self.enforced {
            return false;
        }
        ip_range::hostname_looks_blocked(host, self.restricts_to_link_local(host))
    }
}

fn env_optional(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn env_flag(key: &str, default: bool) -> bool {
    match env_optional(key) {
        Some(value) => {
            let lower = value.to_lowercase();
            !(lower == "false" || lower == "0")
        }
        None => default,
    }
}

pub fn env_list(key: &str) -> Vec<String> {
    env_optional(key)
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

pub fn env_number(key: &str, default: u64) -> u64 {
    env_optional(key)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}

pub fn split_allowed_internal_hosts(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_pasted_host_forms() {
        assert_eq!(
            normalize_allowed_internal_host(" Foo.EXAMPLE.com "),
            "foo.example.com"
        );
        assert_eq!(
            normalize_allowed_internal_host("http://Foo.EXAMPLE.com:8080/path?x=1"),
            "foo.example.com"
        );
        assert_eq!(
            normalize_allowed_internal_host("https://internal.example/issuer"),
            "internal.example"
        );
        assert_eq!(normalize_allowed_internal_host("[::1]"), "::1");
        assert_eq!(normalize_allowed_internal_host("http://[::1]:9000/"), "::1");
        assert_eq!(normalize_allowed_internal_host("10.0.0.5:8080"), "10.0.0.5");
        assert_eq!(normalize_allowed_internal_host("*"), "*");
        assert_eq!(normalize_allowed_internal_host("   "), "");
    }

    #[test]
    fn star_disables_the_guard() {
        let policy = GuardPolicy::new(["*"]);
        assert!(!policy.is_enforced());
        assert!(!policy.is_blocked_hostname("127.0.0.1"));
        assert_eq!(
            policy.check_address("anything", "169.254.169.254"),
            AddressCheck::Allowed
        );
    }

    #[test]
    fn empty_allowlist_blocks_private_ranges() {
        let policy = GuardPolicy::new(Vec::<String>::new());
        assert!(policy.is_enforced());
        assert!(policy.is_blocked_hostname("10.0.0.1"));
        assert!(policy.is_blocked_address("api.example.com", "169.254.169.254"));
        assert!(!policy.is_blocked_address("api.example.com", "8.8.8.8"));
    }

    #[test]
    fn allowlisted_host_opens_private_but_never_link_local() {
        let policy = GuardPolicy::new(["internal.example.com", "http://10.0.0.5:8080"]);
        assert!(policy.is_allowed_internal_host("Internal.Example.COM"));
        assert!(policy.is_allowed_internal_host("internal.example.com"));
        assert!(policy.is_allowed_internal_host("10.0.0.5"));
        assert!(!policy.is_allowed_internal_host("internal.example.com."));
        assert!(!policy.is_blocked_address("internal.example.com", "10.0.0.1"));
        assert!(!policy.is_blocked_address("internal.example.com", "127.0.0.1"));
        assert!(policy.is_blocked_address("internal.example.com", "169.254.169.254"));
        assert!(policy.is_blocked_address("internal.example.com", "fe80::1"));
        assert!(policy.is_blocked_address("other.example.com", "10.0.0.1"));
        assert!(!policy.is_allowed_internal_host("*"));
    }

    #[test]
    fn public_only_policy_ignores_the_instance_allowlist() {
        let policy = GuardPolicy::public_hosts_only();
        assert!(policy.is_blocked_hostname("127.0.0.1"));
        assert!(policy.is_blocked_address("127.0.0.1", "127.0.0.1"));
        assert_eq!(policy.allowed_internal_hosts(), Vec::<String>::new());
    }

    #[test]
    fn bracketed_entries_match_bracketless_connections() {
        let policy = GuardPolicy::new(["[::1]:8080"]);
        assert!(policy.is_allowed_internal_host("::1"));
        assert!(policy.is_allowed_internal_host("[::1]"));
        assert!(!policy.is_blocked_address("[::1]", "::1"));
        assert!(policy.is_blocked_address("[::1]", "fe80::1"));
    }

    #[test]
    fn unvalidatable_host_strings_are_not_treated_as_blocked() {
        let policy = GuardPolicy::new(Vec::<String>::new());
        assert!(!policy.is_blocked_hostname("not-an-ip"));
        assert_eq!(
            policy.check_address("example.com", "not-an-ip"),
            AddressCheck::Unvalidatable
        );
    }

    #[test]
    fn host_normalization_lowercases_for_matching() {
        assert_eq!(normalize_host_for_match("[FE80::1]"), "fe80::1");
        assert_eq!(normalize_host_for_match(" Example.COM "), "example.com");
    }
}
