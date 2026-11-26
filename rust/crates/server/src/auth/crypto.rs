use aes_gcm::aead::{Aead, Payload};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use hkdf::Hkdf;
use rand::RngCore;
use sha2::{Digest, Sha256, Sha512};
use uuid::Uuid;

const ENVELOPE_PREFIX: &str = "enc:v2:";
const NONCE_BYTES: usize = 12;
const TAG_BYTES: usize = 16;
const HKDF_SALT: [u8; 32] = [0u8; 32];

pub fn encryption_key_id(raw_key: &str) -> String {
    let digest = Sha256::digest(raw_key.as_bytes());
    hex::encode(&digest[..4])
}

pub fn derive_gcm_key(raw_key: &str, workspace_id: Option<Uuid>) -> [u8; 32] {
    let info = match workspace_id {
        Some(id) => format!("twenty:enc:v2:{id}"),
        None => "twenty:enc:v2:instance".to_string(),
    };

    let mut key = [0u8; 32];
    Hkdf::<Sha256>::new(Some(&HKDF_SALT), raw_key.as_bytes())
        .expand(info.as_bytes(), &mut key)
        .expect("a 32 byte output is a valid hkdf length");

    key
}

pub fn encrypt_versioned(raw_key: &str, plaintext: &str) -> String {
    encrypt_versioned_for_workspace(raw_key, None, plaintext)
}

pub fn encrypt_versioned_for_workspace(
    raw_key: &str,
    workspace_id: Option<Uuid>,
    plaintext: &str,
) -> String {
    let key = derive_gcm_key(raw_key, workspace_id);
    let cipher = Aes256Gcm::new((&key).into());

    let mut nonce_bytes = [0u8; NONCE_BYTES];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let mut payload = cipher
        .encrypt(
            nonce,
            Payload {
                msg: plaintext.as_bytes(),
                aad: &[],
            },
        )
        .expect("encryption with a fresh key cannot fail");

    let mut sealed = Vec::with_capacity(NONCE_BYTES + payload.len());
    sealed.extend_from_slice(&nonce_bytes);
    sealed.append(&mut payload);

    format!(
        "{}{}:{}",
        ENVELOPE_PREFIX,
        encryption_key_id(raw_key),
        BASE64.encode(sealed)
    )
}

pub fn decrypt_versioned(keys: &[String], envelope: &str) -> Result<String, &'static str> {
    let rest = envelope
        .strip_prefix(ENVELOPE_PREFIX)
        .ok_or("not an enc:v2 envelope")?;

    let (key_id, payload_b64) = rest.split_once(':').ok_or("malformed envelope")?;
    let sealed = BASE64
        .decode(payload_b64)
        .map_err(|_| "malformed payload")?;

    if sealed.len() < NONCE_BYTES + TAG_BYTES {
        return Err("payload too short");
    }

    let (nonce_bytes, ciphertext) = sealed.split_at(NONCE_BYTES);
    let nonce = Nonce::from_slice(nonce_bytes);

    for raw_key in keys {
        if encryption_key_id(raw_key) != key_id {
            continue;
        }

        let key = derive_gcm_key(raw_key, None);
        let cipher = Aes256Gcm::new((&key).into());

        if let Ok(plaintext) = cipher.decrypt(
            nonce,
            Payload {
                msg: ciphertext,
                aad: &[],
            },
        ) {
            return String::from_utf8(plaintext).map_err(|_| "plaintext is not utf-8");
        }
    }

    Err("no encryption key matches the envelope key id")
}

pub fn is_valid_password(password: &str) -> bool {
    let utf16_length = password.encode_utf16().count();
    (8..=50).contains(&utf16_length) && !password.contains('\n')
}

pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    bcrypt::hash(password, 10)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    bcrypt::verify(password, hash)
}

pub fn legacy_jwt_secret(app_secret: &str, body: Option<&Uuid>, token_type: &str) -> String {
    let mut parts = String::with_capacity(app_secret.len() + 36 + token_type.len());

    parts.push_str(app_secret);
    if let Some(id) = body {
        parts.push_str(&id.to_string());
    }
    parts.push_str(token_type);

    hex::encode(Sha256::digest(parts.as_bytes()))
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "used by the test vectors below")
)]
pub fn sha256_hex(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

#[expect(
    dead_code,
    reason = "the legacy AES-256-CTR path lands with stored secrets"
)]
pub fn legacy_aes256_ctr_key(raw_key: &str) -> [u8; 32] {
    let digest = Sha512::digest(raw_key.as_bytes());
    let hex_string = hex::encode(digest);
    let mut key = [0u8; 32];

    for (index, chunk) in hex_string.as_bytes().chunks(2).take(32).enumerate() {
        key[index] = u8::from_str_radix(std::str::from_utf8(chunk).expect("hex is utf-8"), 16)
            .expect("hex digits");
    }

    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_round_trips_and_carries_the_key_id() {
        let raw_key = "a-raw-secret";
        let envelope = encrypt_versioned(raw_key, "the treasure");
        let key_id = encryption_key_id(raw_key);

        assert!(envelope.starts_with(&format!("enc:v2:{key_id}:")));
        assert_eq!(
            decrypt_versioned(&[raw_key.to_string()], &envelope).unwrap(),
            "the treasure"
        );
    }

    #[test]
    fn decrypt_rejects_wrong_keys_and_malformed_envelopes() {
        let envelope = encrypt_versioned("key-a", "data");

        assert!(decrypt_versioned(&["key-b".to_string()], &envelope).is_err());
        assert!(decrypt_versioned(&["key-a".to_string()], "enc:v1:x:y").is_err());
        assert!(decrypt_versioned(&["key-a".to_string()], "garbage").is_err());
    }

    #[test]
    fn password_policy_is_utf16_length_without_newlines() {
        assert!(is_valid_password("longenough1"));
        assert!(!is_valid_password("short1"));
        assert!(!is_valid_password(&"a".repeat(51)));

        let emoji = "😀😀😀😀";
        assert_eq!(emoji.encode_utf16().count(), 8);
        assert!(is_valid_password(emoji));

        assert!(!is_valid_password("has\nnewline12345"));
    }

    #[test]
    fn legacy_jwt_secret_has_no_separator_between_parts() {
        let workspace_id = Uuid::from_u128(1);
        let user_id = Uuid::from_u128(2);

        let expected = {
            use sha2::Digest;
            let input = format!("appsecret{workspace_id}ACCESS");
            hex::encode(Sha256::digest(input.as_bytes()))
        };

        assert_eq!(
            legacy_jwt_secret("appsecret", Some(&workspace_id), "ACCESS"),
            expected
        );

        let expected_user = {
            use sha2::Digest;
            let input = format!("appsecret{user_id}REFRESH");
            hex::encode(Sha256::digest(input.as_bytes()))
        };

        assert_eq!(
            legacy_jwt_secret("appsecret", Some(&user_id), "REFRESH"),
            expected_user
        );
        assert_eq!(
            legacy_jwt_secret("appsecret", None, "WORKSPACE_AGNOSTIC"),
            {
                use sha2::Digest;
                hex::encode(Sha256::digest(b"appsecretWORKSPACE_AGNOSTIC"))
            }
        );
    }

    #[test]
    fn sha256_hex_matches_known_vectors() {
        assert_eq!(
            sha256_hex(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
