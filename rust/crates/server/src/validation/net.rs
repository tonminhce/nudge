use crate::validation::values::js_trim;
use crate::validation::values::utf16_length;

pub fn is_valid_uuid(value: &str) -> bool {
    let bytes = value.as_bytes();

    if bytes.len() != 36 {
        return false;
    }

    let groups = [(0usize, 8usize), (9, 4), (14, 4), (19, 4), (24, 12)];

    for (start, length) in groups {
        for offset in 0..length {
            if !bytes[start + offset].is_ascii_hexdigit() {
                return false;
            }
        }
    }

    for position in [8usize, 13, 18, 23] {
        if bytes[position] != b'-' {
            return false;
        }
    }

    let version = bytes[14].to_ascii_lowercase() as char;

    if !matches!(version, '1' | '2' | '3' | '4' | '5') {
        return false;
    }

    let variant = bytes[19].to_ascii_lowercase() as char;

    matches!(variant, '8' | '9' | 'a' | 'b')
}

pub fn is_valid_uuid_v4(value: &str) -> bool {
    let bytes = value.as_bytes();

    if bytes.len() != 36 {
        return false;
    }

    let groups = [(0usize, 8usize), (9, 4), (14, 4), (19, 4), (24, 12)];

    for (start, length) in groups {
        for offset in 0..length {
            if !bytes[start + offset].is_ascii_hexdigit() {
                return false;
            }
        }
    }

    for position in [8usize, 13, 18, 23] {
        if bytes[position] != b'-' {
            return false;
        }
    }

    bytes[14].to_ascii_lowercase() == b'4'
        && matches!(bytes[19].to_ascii_lowercase(), b'8' | b'9' | b'a' | b'b')
}

fn is_email_code_point(value: char) -> bool {
    !crate::validation::values::is_js_whitespace(value) && value != '@'
}

pub fn is_valid_email(value: &str) -> bool {
    if utf16_length(value) > 255 {
        return false;
    }

    if value.chars().filter(|ch| *ch == '@').count() != 1 {
        return false;
    }

    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };

    let local_count = local.chars().count();
    let domain_count = domain.chars().count();

    if !(1..=64).contains(&local_count) || !(1..=255).contains(&domain_count) {
        return false;
    }

    local.chars().all(|ch| is_email_code_point(ch) && ch != '"')
        && domain.chars().all(is_email_code_point)
}

fn is_label_char(value: char) -> bool {
    value.is_ascii_lowercase() || value.is_ascii_digit() || value == '-'
}

fn matches_label_body(body: &str, maximum: usize) -> bool {
    let count = body.chars().count();

    count >= 1
        && count <= maximum
        && body.chars().all(is_label_char)
        && body
            .chars()
            .next_back()
            .is_some_and(|last| last.is_ascii_alphanumeric())
}

fn matches_repeated_label(label: &str) -> bool {
    if label.is_empty() || label.starts_with('-') {
        return false;
    }

    if matches_label_body(label, 62) {
        return true;
    }

    match label
        .strip_prefix("xn--")
        .or_else(|| label.strip_prefix('_'))
    {
        Some(body) => matches_label_body(body, 62),
        None => false,
    }
}

fn matches_final_label(label: &str) -> bool {
    let starts_alphanumeric = |value: &str| {
        value
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_alphanumeric())
    };

    let plain = label.chars().count() <= 61
        && starts_alphanumeric(label)
        && label.chars().all(is_label_char);

    if plain {
        return true;
    }

    match label.strip_prefix("xn--") {
        Some(body) => {
            body.chars().count() <= 61
                && starts_alphanumeric(body)
                && body.chars().all(is_label_char)
        }
        None => false,
    }
}

fn matches_tld_pair_body(body: &str) -> bool {
    let Some((head, tail)) = body.split_once('.') else {
        return false;
    };

    let head_count = head.chars().count();

    head_count >= 1
        && head_count <= 30
        && head.chars().all(is_label_char)
        && tail.chars().count() >= 2
        && tail.chars().all(|ch| ch.is_ascii_lowercase())
}

fn matches_final_tld_pair(pair: &str) -> bool {
    match pair.strip_prefix("xn--") {
        Some(body) => matches_tld_pair_body(body) || matches_tld_pair_body(pair),
        None => matches_tld_pair_body(pair),
    }
}

fn matches_hostname_regex(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }

    let labels: Vec<&str> = value.split('.').collect();
    let label_count = labels.len();

    if label_count < 2 {
        return false;
    }

    if label_count <= 11
        && labels[..label_count - 1]
            .iter()
            .all(|label| matches_repeated_label(label))
        && matches_final_label(labels[label_count - 1])
    {
        return true;
    }

    label_count >= 3
        && label_count <= 12
        && labels[..label_count - 2]
            .iter()
            .all(|label| matches_repeated_label(label))
        && matches_final_tld_pair(&labels[label_count - 2..].join("."))
}

fn is_ipv4_like(value: &str) -> bool {
    let parts: Vec<&str> = value.split('.').collect();

    parts.len() == 4
        && parts.iter().all(|part| {
            !part.is_empty() && part.len() <= 3 && part.chars().all(|ch| ch.is_ascii_digit())
        })
}

pub fn is_valid_hostname(value: &str, allow_ip: bool, allow_localhost: bool) -> bool {
    let is_ip = is_ipv4_like(value);
    let is_localhost = value == "localhost" || value == "127.0.0.1";

    if is_localhost && !allow_localhost {
        return false;
    }

    if is_ip && !allow_ip {
        return false;
    }

    matches_hostname_regex(value) || is_localhost || is_ip
}

fn strip_scheme(value: &str) -> &str {
    let bytes = value.as_bytes();

    if bytes.is_empty() || !bytes[0].is_ascii_alphabetic() {
        return value;
    }

    let mut index = 0;

    while index < bytes.len() {
        let ch = bytes[index];

        if ch == b':' {
            if index + 3 <= bytes.len() && bytes[index + 1] == b'/' && bytes[index + 2] == b'/' {
                return &value[index + 3..];
            }

            return value;
        }

        if !(ch.is_ascii_alphanumeric() || ch == b'+' || ch == b'.' || ch == b'-') {
            return value;
        }

        index += 1;
    }

    value
}

fn strip_port(value: &str) -> &str {
    let Some((head, tail)) = value.rsplit_once(':') else {
        return value;
    };

    if !tail.is_empty() && tail.chars().all(|ch| ch.is_ascii_digit()) {
        return head;
    }

    value
}

fn strip_user_info(value: &str) -> &str {
    match value.rfind('@') {
        Some(index) => &value[index + 1..],
        None => value,
    }
}

fn strip_www_and_trailing_dots(host: &str) -> String {
    let mut labels: Vec<&str> = host.split('.').collect();

    while labels.last().is_some_and(|label| label.is_empty()) {
        labels.pop();
    }

    while labels.first().is_some_and(|label| *label == "www") {
        labels.remove(0);
    }

    labels.join(".")
}

pub fn normalize_domain(raw_domain: &str) -> String {
    let trimmed = js_trim(raw_domain);
    let without_scheme = strip_scheme(trimmed);
    let pathless = without_scheme
        .split(['/', '\\', '?', '#'])
        .next()
        .unwrap_or_default();
    let without_port = strip_port(strip_user_info(pathless));
    let host = without_port.to_ascii_lowercase();

    strip_www_and_trailing_dots(&host)
}

pub fn is_valid_domain(raw_domain: &str) -> bool {
    let normalized = normalize_domain(raw_domain);

    if normalized.is_empty() {
        return false;
    }

    is_valid_hostname(&normalized, false, false)
}

fn split_scheme(url: &str) -> Option<(String, &str)> {
    let separator = url.find(':')?;

    if separator == 0 {
        return None;
    }

    let scheme = &url[..separator];

    if !scheme.as_bytes()[0].is_ascii_alphabetic() {
        return None;
    }

    if !scheme
        .bytes()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == b'+' || ch == b'.' || ch == b'-')
    {
        return None;
    }

    Some((scheme.to_ascii_lowercase(), &url[separator + 1..]))
}

fn is_forbidden_host_char(value: char) -> bool {
    matches!(
        value,
        '\0'..='\u{20}'
            | '#'
            | '%'
            | '/'
            | ':'
            | '<'
            | '>'
            | '?'
            | '@'
            | '['
            | '\\'
            | ']'
            | '^'
            | '|'
    )
}

fn strip_url_credentials(host: &str) -> &str {
    match host.rfind('@') {
        Some(index) => &host[index + 1..],
        None => host,
    }
}

fn matches_host_authority(value: &str) -> bool {
    let authority = value
        .split(['/', '\\', '?', '#'])
        .next()
        .unwrap_or_default();
    let host = strip_url_credentials(authority);

    let host = match host.strip_prefix('[') {
        Some(rest) => match rest.find(']') {
            Some(index) => {
                return rest[..index]
                    .chars()
                    .all(|ch| ch.is_ascii_hexdigit() || ch == ':')
            }
            None => return false,
        },
        None => host,
    };

    let host = match host.rsplit_once(':') {
        Some((head, port)) => {
            if port.is_empty() || !port.chars().all(|ch| ch.is_ascii_digit()) {
                return false;
            }

            if port.parse::<u32>().ok().is_none_or(|value| value > 65_535) {
                return false;
            }

            head
        }
        None => host,
    };

    !host.is_empty() && !host.chars().any(is_forbidden_host_char)
}

pub fn is_safe_url(url: &str) -> bool {
    let trimmed = url.trim_matches(|ch: char| ch <= '\u{20}');
    let cleaned: String = trimmed
        .chars()
        .filter(|ch| !matches!(ch, '\t' | '\n' | '\r'))
        .collect();

    if cleaned.starts_with('/') {
        return !cleaned.starts_with("//") && !cleaned.starts_with("/\\");
    }

    let Some((scheme, rest)) = split_scheme(&cleaned) else {
        return false;
    };

    match scheme.as_str() {
        "mailto" | "tel" => true,
        "http" | "https" => matches_host_authority(rest),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_guard_accepts_versions_one_to_five_only() {
        assert!(is_valid_uuid("3f2504e0-4f89-11d3-9a0c-0305e82c3301"));
        assert!(is_valid_uuid("3f2504e0-4f89-41d3-9a0c-0305e82c3301"));
        assert!(is_valid_uuid("3F2504E0-4F89-11D3-9A0C-0305E82C3301"));
        assert!(!is_valid_uuid("00000000-0000-0000-0000-000000000000"));
        assert!(!is_valid_uuid("3f2504e0-4f89-61d3-9a0c-0305e82c3301"));
        assert!(!is_valid_uuid("3f2504e0-4f89-41d3-ca0c-0305e82c3301"));
        assert!(!is_valid_uuid("not-a-uuid"));
        assert!(is_valid_uuid_v4("3f2504e0-4f89-41d3-9a0c-0305e82c3301"));
        assert!(!is_valid_uuid_v4("3f2504e0-4f89-11d3-9a0c-0305e82c3301"));
        assert!(!is_valid_uuid_v4(
            "urn:uuid:3f2504e0-4f89-41d3-9a0c-0305e82c3301"
        ));
    }

    #[test]
    fn email_guard_mirrors_the_unicode_email_pattern() {
        for accepted in [
            "user@example.com",
            "user.name+tag@sub.example.co",
            "a@b.co",
            "user@domain",
            "user@domain.c",
            "us er@example.com",
            "user@x\".com",
            "user@.com",
            "user@com.",
            "a..b@x.com",
            ".a@x.com",
            "a.@x.com",
            "jöhn@example.com",
        ] {
            assert!(is_valid_email(accepted), "expected {accepted} to pass");
        }

        for rejected in [
            "",
            "not-an-email",
            "john@",
            "@example.com",
            "user@@example.com",
            "us\"er@x.com",
            "john doe@x.com",
            "a\u{a0}b@x.com",
            "ab@cd\u{2028}ef.com",
            "user@x\u{3000}.com",
            "user@example.com ",
            &format!("{}@x.com", "a".repeat(65)),
        ] {
            assert!(!is_valid_email(rejected), "expected {rejected} to fail");
        }

        assert!(is_valid_email(&format!("{}@x.com", "a".repeat(64))));
        assert!(!is_valid_email(&"x".repeat(253)));
        assert!(!is_valid_email(&format!(
            "{}@{}",
            "a".repeat(64),
            "b".repeat(200)
        )));
        assert!(is_valid_email("a\u{85}b@x.com"));
    }

    #[test]
    fn domain_normalization_strips_scheme_port_www_and_dots() {
        assert_eq!(
            normalize_domain("https://www.Twenty.com/path"),
            "twenty.com"
        );
        assert_eq!(normalize_domain("http://localhost:3000"), "localhost");
        assert_eq!(normalize_domain("twenty.com."), "twenty.com");
        assert_eq!(normalize_domain("user:pass@twenty.com:8080"), "twenty.com");
        assert_eq!(normalize_domain("www.www.twenty.com"), "twenty.com");
        assert_eq!(normalize_domain("twenty.com:12345678"), "twenty.com");
        assert_eq!(normalize_domain("twenty.com:1234x"), "twenty.com:1234x");
        assert_eq!(normalize_domain("ab@cd@ef.com"), "ef.com");
        assert_eq!(normalize_domain("   "), "");
        assert_eq!(normalize_domain("twenty#frag"), "twenty");
    }

    #[test]
    fn domain_guard_rejects_localhost_ips_and_bare_hosts() {
        for accepted in [
            "twenty.com",
            "sub.twenty.com",
            "twenty.co",
            "t.c",
            "TWENTY.COM",
            "_x.com",
            "xn--bcher-kva.de",
            "a.-b.com",
            "256.256.256.256",
            "a.b.c.d.e.f.g.h.i.j.k",
        ] {
            assert!(is_valid_domain(accepted), "expected {accepted} to pass");
        }

        for rejected in [
            "localhost",
            "127.0.0.1",
            "twenty",
            "",
            "twenty.com:1234x",
            "a.b.c.d.e.f.g.h.i.j.k.l",
            "-bad.com",
            "bad-.com",
            "x_n.com",
            "a..b.com",
            ".a.com",
            &format!("{}a.com", "a".repeat(70)),
            &format!("{}.com", "x".repeat(63)),
        ] {
            assert!(!is_valid_domain(rejected), "expected {rejected} to fail");
        }

        assert!(is_valid_hostname("localhost", true, true));
        assert!(!is_valid_hostname("localhost", true, false));
        assert!(!is_valid_hostname("127.0.0.1", false, true));
        assert!(!is_valid_hostname("LOCALHOST", true, true));
    }

    #[test]
    fn safe_url_guard_blocks_scripting_protocols() {
        for accepted in [
            "https://twenty.com",
            "http://twenty.com",
            "mailto:hey@twenty.com",
            "tel:+1234",
            "/relative/path",
            "HTTP://TWENTY.COM",
            "  https://a.com  ",
            "http:x",
            "http:/a",
            "http://user:pw@a.b.c",
            "/a b",
            "https://a.com?q=1#f",
            "tel:",
        ] {
            assert!(is_safe_url(accepted), "expected {accepted} to pass");
        }

        for rejected in [
            "javascript:alert(1)",
            "data:text/html;base64,PHNjcmlwdD4=",
            "//evil.com",
            "/\\evil.com",
            "not a url",
            "https:",
            "http://",
            "ftp://a.com",
            "#frag",
            "blob:https://a/b",
            "1http://a.com",
            "http://a.com:99999",
            "http://a.com:x",
        ] {
            assert!(!is_safe_url(rejected), "expected {rejected} to fail");
        }
    }

    #[test]
    fn repeated_labels_enforce_the_prefix_and_length_alternatives() {
        assert!(matches_repeated_label("a"));
        assert!(matches_repeated_label("a1"));
        assert!(matches_repeated_label("ab-c"));
        assert!(matches_repeated_label("1"));
        assert!(matches_repeated_label("_a"));
        assert!(matches_repeated_label("xn--a"));
        assert!(!matches_repeated_label(""));
        assert!(!matches_repeated_label("-a"));
        assert!(!matches_repeated_label("a-"));
        assert!(!matches_repeated_label("a_b"));
        assert!(!matches_repeated_label(&"a".repeat(63)));
        assert!(matches_repeated_label(&"a".repeat(62)));
        assert!(matches_repeated_label(&format!("xn--{}", "a".repeat(62))));
        assert!(!matches_repeated_label(&format!("xn--{}", "a".repeat(63))));
    }

    #[test]
    fn final_labels_allow_the_short_tld_alternative() {
        assert!(matches_final_label("com"));
        assert!(matches_final_label("c"));
        assert!(matches_final_label("xn--abc"));
        assert!(!matches_final_label("-a"));
        assert!(!matches_final_label(&"a".repeat(62)));
        assert!(matches_final_label(&"a".repeat(61)));
        assert!(matches_final_tld_pair("a-..com"));
        assert!(matches_final_tld_pair("-b.com"));
        assert!(!matches_final_tld_pair("a.c"));
        assert!(!matches_final_tld_pair("a.b1c"));
    }
}
