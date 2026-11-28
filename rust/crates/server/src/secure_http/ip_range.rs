use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CidrV4 {
    pub network: u32,
    pub prefix: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CidrV6 {
    pub network: u128,
    pub prefix: u8,
}

pub const PRIVATE_RANGES_V4: &[CidrV4] = &[
    CidrV4 {
        network: 0x0000_0000,
        prefix: 8,
    },
    CidrV4 {
        network: 0x0a00_0000,
        prefix: 8,
    },
    CidrV4 {
        network: 0x6440_0000,
        prefix: 10,
    },
    CidrV4 {
        network: 0x7f00_0000,
        prefix: 8,
    },
    CidrV4 {
        network: 0xa9fe_0000,
        prefix: 16,
    },
    CidrV4 {
        network: 0xac10_0000,
        prefix: 12,
    },
    CidrV4 {
        network: 0xc000_0000,
        prefix: 24,
    },
    CidrV4 {
        network: 0xc000_0200,
        prefix: 24,
    },
    CidrV4 {
        network: 0xc0a8_0000,
        prefix: 16,
    },
    CidrV4 {
        network: 0xc612_0000,
        prefix: 15,
    },
    CidrV4 {
        network: 0xc633_6400,
        prefix: 24,
    },
    CidrV4 {
        network: 0xcb00_7100,
        prefix: 24,
    },
    CidrV4 {
        network: 0xe000_0000,
        prefix: 4,
    },
    CidrV4 {
        network: 0xf000_0000,
        prefix: 4,
    },
];

pub const PRIVATE_RANGES_V6: &[CidrV6] = &[
    CidrV6 {
        network: 0x0000_0000_0000_0000_0000_0000_0000_0001,
        prefix: 128,
    },
    CidrV6 {
        network: 0x0000_0000_0000_0000_0000_0000_0000_0000,
        prefix: 128,
    },
    CidrV6 {
        network: 0xfc00_0000_0000_0000_0000_0000_0000_0000,
        prefix: 7,
    },
    CidrV6 {
        network: 0xfe80_0000_0000_0000_0000_0000_0000_0000,
        prefix: 10,
    },
];

pub const LINK_LOCAL_RANGES_V4: &[CidrV4] = &[CidrV4 {
    network: 0xa9fe_0000,
    prefix: 16,
}];

pub const LINK_LOCAL_RANGES_V6: &[CidrV6] = &[CidrV6 {
    network: 0xfe80_0000_0000_0000_0000_0000_0000_0000,
    prefix: 10,
}];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressCheck {
    Allowed,
    Blocked,
    Unvalidatable,
}

impl AddressCheck {
    pub fn blocked(self) -> bool {
        self == AddressCheck::Blocked
    }
}

fn v4_mask(prefix: u8) -> u32 {
    match prefix {
        0 => 0,
        32..=u8::MAX => u32::MAX,
        bits => u32::MAX << (32 - u32::from(bits)),
    }
}

fn v6_mask(prefix: u8) -> u128 {
    match prefix {
        0 => 0,
        128..=u8::MAX => u128::MAX,
        bits => u128::MAX << (128 - u128::from(bits)),
    }
}

fn matches_v4(addr: Ipv4Addr, ranges: &[CidrV4]) -> bool {
    let value = u32::from(addr);
    ranges
        .iter()
        .any(|range| value & v4_mask(range.prefix) == range.network & v4_mask(range.prefix))
}

fn matches_v6(addr: Ipv6Addr, ranges: &[CidrV6]) -> bool {
    let value = u128::from(addr);
    ranges
        .iter()
        .any(|range| value & v6_mask(range.prefix) == range.network & v6_mask(range.prefix))
}

pub fn is_private_addr(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(v4) => matches_v4(v4, PRIVATE_RANGES_V4),
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(mapped) => matches_v4(mapped, PRIVATE_RANGES_V4),
            None => matches_v6(v6, PRIVATE_RANGES_V6),
        },
    }
}

pub fn is_link_local_addr(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(v4) => matches_v4(v4, LINK_LOCAL_RANGES_V4),
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(mapped) => matches_v4(mapped, LINK_LOCAL_RANGES_V4),
            None => matches_v6(v6, LINK_LOCAL_RANGES_V6),
        },
    }
}

fn normalize_octet(part: &str) -> Option<u64> {
    let lower = part.to_ascii_lowercase();
    if let Some(hex) = lower.strip_prefix("0x") {
        if hex.is_empty() || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        return u64::from_str_radix(hex, 16).ok();
    }
    if part.starts_with('0') && part.len() > 1 && part.chars().all(|c| matches!(c, '0'..='7')) {
        return u64::from_str_radix(part, 8).ok();
    }
    if part == "0" || part.starts_with(|c: char| matches!(c, '1'..='9')) {
        return part.parse::<u64>().ok();
    }
    None
}

pub fn normalize_ipv4(addr: &str) -> Option<Ipv4Addr> {
    let mut values = Vec::new();
    for part in addr.split('.') {
        values.push(normalize_octet(part)?);
    }
    let value = match values.as_slice() {
        [one] => *one,
        [first, second] => {
            if *first > 0xff || *second > 0xff_ffff {
                return None;
            }
            (first << 24) | (second & 0xff_ffff)
        }
        [first, second, third] => {
            if *first > 0xff || *second > 0xff || *third > 0xffff {
                return None;
            }
            (first << 24) | (second << 16) | (third & 0xffff)
        }
        [first, second, third, fourth] => {
            if values.iter().any(|value| *value > 0xff) {
                return None;
            }
            (first << 24) | (second << 16) | (third << 8) | fourth
        }
        _ => return None,
    };
    Some(Ipv4Addr::from(value as u32))
}

pub fn strip_ipv6_brackets(host: &str) -> &str {
    host.strip_prefix('[')
        .and_then(|inner| inner.strip_suffix(']'))
        .unwrap_or(host)
}

pub fn parse_host_address(host: &str) -> Option<IpAddr> {
    let bare = strip_ipv6_brackets(host);
    if bare.contains(':') {
        return Ipv6Addr::from_str(bare).ok().map(IpAddr::V6);
    }
    normalize_ipv4(bare)
        .map(IpAddr::V4)
        .or_else(|| Ipv4Addr::from_str(bare).ok().map(IpAddr::V4))
}

pub fn check_address(host: &str, restricted_to_link_local: bool) -> AddressCheck {
    let bare = strip_ipv6_brackets(host);
    if bare.contains(':') {
        return match Ipv6Addr::from_str(bare).ok().map(IpAddr::V6) {
            Some(addr) if is_blocked_addr(addr, restricted_to_link_local) => AddressCheck::Blocked,
            Some(_) => AddressCheck::Allowed,
            None => AddressCheck::Allowed,
        };
    }
    match parse_host_address(bare) {
        Some(addr) if is_blocked_addr(addr, restricted_to_link_local) => AddressCheck::Blocked,
        Some(_) => AddressCheck::Allowed,
        None => AddressCheck::Unvalidatable,
    }
}

fn is_blocked_addr(addr: IpAddr, restricted_to_link_local: bool) -> bool {
    if restricted_to_link_local {
        is_link_local_addr(addr)
    } else {
        is_private_addr(addr)
    }
}

pub fn is_private_ip(address: &str) -> Result<bool, AddressCheck> {
    match check_address(address, false) {
        AddressCheck::Allowed => Ok(false),
        AddressCheck::Blocked => Ok(true),
        AddressCheck::Unvalidatable => Err(AddressCheck::Unvalidatable),
    }
}

pub fn is_link_local_ip(address: &str) -> Result<bool, AddressCheck> {
    match check_address(address, true) {
        AddressCheck::Allowed => Ok(false),
        AddressCheck::Blocked => Ok(true),
        AddressCheck::Unvalidatable => Err(AddressCheck::Unvalidatable),
    }
}

pub fn hostname_looks_blocked(address: &str, restricted_to_link_local: bool) -> bool {
    check_address(address, restricted_to_link_local).blocked()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_every_blocked_ipv4_class() {
        let blocked = [
            "0.0.0.0",
            "0.0.0.1",
            "0.255.255.255",
            "10.0.0.1",
            "10.255.255.255",
            "100.64.0.0",
            "100.127.255.255",
            "127.0.0.1",
            "127.0.0.2",
            "127.255.255.255",
            "169.254.0.1",
            "169.254.169.254",
            "172.16.0.1",
            "172.31.255.255",
            "192.0.0.1",
            "192.0.2.1",
            "192.168.0.1",
            "192.168.255.255",
            "198.18.0.1",
            "198.19.255.255",
            "198.51.100.1",
            "203.0.113.1",
            "224.0.0.1",
            "239.255.255.255",
            "240.0.0.1",
            "255.255.255.254",
        ];
        for address in blocked {
            assert!(is_private_ip(address).unwrap_or(false), "{address} blocked");
        }
    }

    #[test]
    fn allows_public_and_adjacent_addresses() {
        let allowed = [
            "100.63.255.255",
            "100.128.0.0",
            "11.0.0.1",
            "172.15.0.1",
            "172.32.0.1",
            "192.167.1.1",
            "198.20.0.0",
            "1.1.1.1",
            "8.8.8.8",
            "93.184.216.34",
        ];
        for address in allowed {
            assert!(!is_private_ip(address).unwrap_or(true), "{address} allowed");
        }
    }

    #[test]
    fn blocks_ipv6_classes() {
        for address in ["::1", "::", "fc00::1", "fd00::1", "fe80::1"] {
            assert!(is_private_ip(address).unwrap_or(false), "{address} blocked");
        }
        assert!(!is_private_ip("2001:4860:4860::8888").unwrap_or(true));
    }

    #[test]
    fn blocks_mapped_ipv6_forms() {
        let blocked = [
            "::ffff:127.0.0.1",
            "::ffff:169.254.169.254",
            "::ffff:10.0.0.1",
            "::ffff:192.168.1.1",
            "::ffff:172.16.0.1",
            "::ffff:7f00:1",
            "::ffff:a9fe:a9fe",
            "::ffff:a00:1",
            "::ffff:c0a8:101",
            "::ffff:ac10:1",
            "::ffff:0:0",
            "0:0:0:0:0:ffff:7f00:1",
        ];
        for address in blocked {
            assert!(is_private_ip(address).unwrap_or(false), "{address} blocked");
        }
        assert!(!is_private_ip("::ffff:808:808").unwrap_or(true));
        assert!(!is_private_ip("::ffff:8.8.8.8").unwrap_or(true));
    }

    #[test]
    fn blocks_encoded_ipv4_forms() {
        for address in [
            "2130706433",
            "0x7f.0.0.1",
            "0177.0.0.1",
            "0x7f000001",
            "0",
            "127.1",
            "10.1",
        ] {
            assert!(is_private_ip(address).unwrap_or(false), "{address} blocked");
        }
    }

    #[test]
    fn out_of_range_forms_are_unvalidatable() {
        assert_eq!(normalize_ipv4("256.1.1.1"), None);
        assert_eq!(normalize_ipv4("1.256.1.1"), None);
        assert_eq!(normalize_ipv4("08"), None);
        assert_eq!(normalize_ipv4("1.2.3.4.5"), None);
        assert_eq!(normalize_ipv4("255.255.255.255.1"), None);
        assert_eq!(is_private_ip("invalid"), Err(AddressCheck::Unvalidatable));
        assert_eq!(is_private_ip(""), Err(AddressCheck::Unvalidatable));
    }

    #[test]
    fn link_local_range_stays_closed_for_allowlisted_hosts() {
        for address in [
            "169.254.169.254",
            "169.254.0.1",
            "fe80::1",
            "::ffff:a9fe:a9fe",
        ] {
            assert!(
                is_link_local_ip(address).unwrap_or(false),
                "{address} blocked"
            );
        }
        for address in ["10.0.0.1", "127.0.0.1", "::1", "8.8.8.8", "fd00::1"] {
            assert!(
                !is_link_local_ip(address).unwrap_or(true),
                "{address} allowed"
            );
        }
    }

    #[test]
    fn parses_bracketed_and_plain_literals() {
        assert_eq!(
            parse_host_address("[::1]"),
            Some(IpAddr::V6(Ipv6Addr::LOCALHOST))
        );
        assert_eq!(
            parse_host_address("127.0.0.1"),
            Some(IpAddr::V4(Ipv4Addr::LOCALHOST))
        );
        assert_eq!(parse_host_address("example.com"), None);
    }

    #[test]
    fn unparseable_ipv6_strings_are_not_blocked() {
        assert!(!is_private_ip(":::not-an-ip").unwrap_or(true));
    }
}
