use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::Sha256;

pub const HEADER_TIMESTAMP: &str = "X-Twenty-Webhook-Timestamp";
pub const HEADER_SIGNATURE: &str = "X-Twenty-Webhook-Signature";
pub const HEADER_NONCE: &str = "X-Twenty-Webhook-Nonce";

type HmacSha256 = Hmac<Sha256>;

pub fn generate_signature(secret: &str, timestamp: &str, payload_json: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts keys of any length");
    mac.update(timestamp.as_bytes());
    mac.update(b":");
    mac.update(payload_json.as_bytes());

    hex::encode(mac.finalize().into_bytes())
}

pub fn generate_webhook_secret() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);

    hex::encode(bytes)
}

pub fn generate_nonce() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);

    hex::encode(bytes)
}

pub fn timestamp_millis(now: std::time::SystemTime) -> String {
    now.duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is after the unix epoch")
        .as_millis()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_webhook_secret_is_64_hex_chars() {
        let secret = generate_webhook_secret();
        assert_eq!(secret.len(), 64);
        assert!(secret.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn nonce_is_32_hex_chars() {
        let nonce = generate_nonce();
        assert_eq!(nonce.len(), 32);
        assert!(nonce.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn timestamp_millis_matches_epoch_milliseconds() {
        let now =
            std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_millis(1_751_450_400_123);

        assert_eq!(timestamp_millis(now), "1751450400123");
    }
}
