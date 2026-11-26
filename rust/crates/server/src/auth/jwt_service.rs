use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use p256::ecdsa::signature::{Signer, Verifier};
use p256::ecdsa::{Signature, SigningKey, VerifyingKey};
use p256::pkcs8::{DecodePrivateKey, DecodePublicKey};
use serde_json::{json, Value};
use sha2::Sha256;
use std::sync::Arc;
use uuid::Uuid;

use super::crypto;
use super::error::{AuthError, AuthResult};
use super::signing_keys::SigningKeyManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    Access,
    Refresh,
    WorkspaceAgnostic,
    Login,
    ApiKey,
    Playground,
    File,
    FileUpload,
}

impl TokenType {
    pub fn as_str(self) -> &'static str {
        match self {
            TokenType::Access => "ACCESS",
            TokenType::Refresh => "REFRESH",
            TokenType::WorkspaceAgnostic => "WORKSPACE_AGNOSTIC",
            TokenType::Login => "LOGIN",
            TokenType::ApiKey => "API_KEY",
            TokenType::Playground => "PLAYGROUND",
            TokenType::File => "FILE",
            TokenType::FileUpload => "FILE_UPLOAD",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        Some(match raw {
            "ACCESS" => TokenType::Access,
            "REFRESH" => TokenType::Refresh,
            "WORKSPACE_AGNOSTIC" => TokenType::WorkspaceAgnostic,
            "LOGIN" => TokenType::Login,
            "API_KEY" => TokenType::ApiKey,
            "PLAYGROUND" => TokenType::Playground,
            "FILE" => TokenType::File,
            "FILE_UPLOAD" => TokenType::FileUpload,
            _ => return None,
        })
    }
}

pub struct JwtService {
    app_secret: String,
    signing_keys: Arc<SigningKeyManager>,
}

impl JwtService {
    pub fn new(app_secret: String, signing_keys: Arc<SigningKeyManager>) -> Self {
        Self {
            app_secret,
            signing_keys,
        }
    }

    pub async fn sign(
        &self,
        claims: Value,
        token_type: TokenType,
        expires_in_secs: u64,
        jwt_id: Option<&Uuid>,
    ) -> AuthResult<String> {
        let mut payload = claims;

        let object = payload
            .as_object_mut()
            .ok_or_else(|| AuthError::internal("jwt claims must be an object"))?;

        object.insert("type".to_string(), json!(token_type.as_str()));

        if let Some(jwt_id) = jwt_id {
            object.insert("jti".to_string(), json!(jwt_id.to_string()));
        }

        let now = chrono::Utc::now().timestamp();
        object.insert("iat".to_string(), json!(now));
        object.insert("exp".to_string(), json!(now + expires_in_secs as i64));

        let key = self.signing_keys.current_key().await?;

        let signing_key = SigningKey::from_pkcs8_pem(&key.private_key_pem)
            .map_err(|error| AuthError::internal(error.to_string()))?;

        let header = json!({
            "alg": "ES256",
            "typ": "JWT",
            "kid": key.id.to_string(),
        });

        let encoded_header = URL_SAFE_NO_PAD.encode(header.to_string());
        let encoded_payload = URL_SAFE_NO_PAD.encode(payload.to_string());
        let signing_input = format!("{encoded_header}.{encoded_payload}");

        let signature: Signature = signing_key.sign(signing_input.as_bytes());

        Ok(format!(
            "{signing_input}.{}",
            URL_SAFE_NO_PAD.encode(signature.to_bytes())
        ))
    }

    pub fn decode_payload(token: &str) -> Option<Value> {
        let parts: Vec<&str> = token.split('.').collect();

        if parts.len() != 3 {
            return None;
        }

        let bytes = URL_SAFE_NO_PAD.decode(parts[1]).ok()?;

        serde_json::from_slice(&bytes).ok()
    }

    pub async fn verify(&self, token: &str) -> AuthResult<Value> {
        let (encoded_header, encoded_payload, encoded_signature) = split_token(token)?;
        let header: Value = decode_json(encoded_header)?;

        let algorithm = header
            .get("alg")
            .and_then(Value::as_str)
            .ok_or_else(|| AuthError::unauthenticated("Token invalid."))?
            .to_string();

        let kid = if algorithm == "ES256" {
            Some(
                header
                    .get("kid")
                    .and_then(Value::as_str)
                    .and_then(|kid| Uuid::parse_str(kid).ok())
                    .ok_or_else(|| AuthError::unauthenticated("Token invalid."))?,
            )
        } else {
            None
        };

        let public_key_pem = match kid {
            Some(kid) => Some(self.signing_keys.public_key_for_kid(&kid).await?),
            None => None,
        };

        let payload = verify_signature_parts(
            &algorithm,
            encoded_header,
            encoded_payload,
            encoded_signature,
            &self.app_secret,
            public_key_pem.as_deref(),
        )?;

        if let Some(exp) = payload.get("exp").and_then(Value::as_i64) {
            if chrono::Utc::now().timestamp() >= exp {
                return Err(AuthError::unauthenticated("Token has expired."));
            }
        }

        Ok(payload)
    }

    pub async fn verify_typed(&self, token: &str, expected: TokenType) -> AuthResult<Value> {
        let payload = self.verify(token).await?;

        let token_type = payload
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| AuthError::unauthenticated("Token invalid."))?;

        if token_type != expected.as_str() {
            return Err(AuthError::unauthenticated("Token invalid."));
        }

        Ok(payload)
    }
}

fn split_token(token: &str) -> AuthResult<(&str, &str, &str)> {
    let mut parts = token.split('.');

    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(a), Some(b), Some(c), None) => Ok((a, b, c)),
        _ => Err(AuthError::unauthenticated("Token invalid.")),
    }
}

fn decode_json(encoded: &str) -> AuthResult<Value> {
    let bytes = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| AuthError::unauthenticated("Token invalid."))?;

    serde_json::from_slice(&bytes).map_err(|_| AuthError::unauthenticated("Token invalid."))
}

fn verify_signature_parts(
    algorithm: &str,
    encoded_header: &str,
    encoded_payload: &str,
    encoded_signature: &str,
    app_secret: &str,
    public_key_pem: Option<&str>,
) -> AuthResult<Value> {
    let signing_input = format!("{encoded_header}.{encoded_payload}");

    let signature_bytes = URL_SAFE_NO_PAD
        .decode(encoded_signature)
        .map_err(|_| AuthError::unauthenticated("Token invalid."))?;

    if algorithm == "ES256" {
        let signature = Signature::from_slice(&signature_bytes)
            .map_err(|_| AuthError::unauthenticated("Token invalid."))?;

        let public_key_pem =
            public_key_pem.ok_or_else(|| AuthError::unauthenticated("Token invalid."))?;

        let verifying_key = VerifyingKey::from_public_key_pem(public_key_pem)
            .map_err(|_| AuthError::unauthenticated("Token invalid."))?;

        verifying_key
            .verify(signing_input.as_bytes(), &signature)
            .map_err(|_| AuthError::unauthenticated("Token invalid."))?;
    } else if algorithm == "HS256" {
        let payload: Value = decode_json(encoded_payload)?;

        let body = payload
            .get("workspaceId")
            .and_then(Value::as_str)
            .and_then(|raw| Uuid::parse_str(raw).ok());

        let body = match body {
            Some(id) => Some(id),
            None => payload
                .get("userId")
                .and_then(Value::as_str)
                .and_then(|raw| Uuid::parse_str(raw).ok()),
        };

        let Some(body) = body else {
            return Err(AuthError::unauthenticated("Token invalid."));
        };

        let token_type = payload
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| AuthError::unauthenticated("Token invalid."))?;

        let secret = crypto::legacy_jwt_secret(app_secret, Some(&body), token_type);

        let mut mac =
            Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("hmac accepts any key length");
        mac.update(signing_input.as_bytes());

        mac.verify_slice(&signature_bytes)
            .map_err(|_| AuthError::unauthenticated("Token invalid."))?;
    } else {
        return Err(AuthError::unauthenticated("Token invalid."));
    }

    decode_json(encoded_payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_types_round_trip_the_node_enum_strings() {
        for token_type in [
            TokenType::Access,
            TokenType::Refresh,
            TokenType::WorkspaceAgnostic,
            TokenType::Login,
            TokenType::ApiKey,
            TokenType::Playground,
            TokenType::File,
            TokenType::FileUpload,
        ] {
            assert_eq!(TokenType::parse(token_type.as_str()), Some(token_type));
        }

        assert_eq!(TokenType::parse("NOPE"), None);
    }

    #[test]
    fn split_token_rejects_malformed_parts() {
        assert!(split_token("not-a-token").is_err());
        assert!(split_token("a.b.c.d").is_err());
        assert!(split_token("a.b.c").is_ok());
    }

    #[test]
    fn legacy_hs256_secret_matches_the_node_derivation_vector() {
        let workspace_id = Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap();

        assert_eq!(
            crypto::legacy_jwt_secret("test-app-secret", Some(&workspace_id), "ACCESS"),
            "d4c9342c2b10e259bba5c386ff178582d25d748566c7b4daf3cdb402cce338ee"
        );
    }

    #[test]
    fn node_signed_legacy_hs256_token_verifies() {
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMTExMTExMS0yMjIyLTMzMzMtNDQ0NC01NTU1NTU1NTU1NTUiLCJ1c2VySWQiOiIxMTExMTExMS0yMjIyLTMzMzMtNDQ0NC01NTU1NTU1NTU1NTUiLCJ3b3Jrc3BhY2VJZCI6IjExMTExMTExLTIyMjItMzMzMy00NDQ0LTU1NTU1NTU1NTU1NSIsInR5cGUiOiJBQ0NFU1MiLCJleHAiOjQxMDI0NDQ4MDB9.BE0EqRw9caEcAk4HA4-gskv85L8v6wK0E3JMgpK7P4Q";

        let (header, payload, signature) = split_token(token).unwrap();

        let verified =
            verify_signature_parts("HS256", header, payload, signature, "test-app-secret", None)
                .unwrap();

        assert_eq!(verified.get("type").and_then(Value::as_str), Some("ACCESS"));
    }

    #[test]
    fn node_signed_es256_token_verifies_with_the_same_public_key() {
        let public_pem = "-----BEGIN PUBLIC KEY-----\nMFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEgGNb5O01AJi2tRHzlYg5TkO5rDBa\n3AlwkDoRwITNPlZLpc4d0waxgGf8srfcZG1vggeTGHcum6gtjoQz8bRKcA==\n-----END PUBLIC KEY-----\n";

        let token = "eyJhbGciOiJFUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyLTEiLCJ0eXBlIjoiQUNDRVNTIiwiZXhwIjo0MTAyNDQ0ODAwfQ.vhrE5RtXvt_YnbCrOlnrCurw2O820fEqWw6J_X51WpIOj_f0Mst6rxvy_t7xf8_DD7Vt_V4zCiX7JJxSCOz9Dg";

        let (header, payload, signature) = split_token(token).unwrap();

        let verified = verify_signature_parts(
            "ES256",
            header,
            payload,
            signature,
            "test-app-secret",
            Some(public_pem),
        )
        .unwrap();

        assert_eq!(verified.get("sub").and_then(Value::as_str), Some("user-1"));
    }

    #[test]
    fn tampered_node_tokens_reject() {
        let public_pem = "-----BEGIN PUBLIC KEY-----\nMFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEgGNb5O01AJi2tRHzlYg5TkO5rDBa\n3AlwkDoRwITNPlZLpc4d0waxgGf8srfcZG1vggeTGHcum6gtjoQz8bRKcA==\n-----END PUBLIC KEY-----\n";

        let token = "eyJhbGciOiJFUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyLTEiLCJ0eXBlIjoiQUNDRVNTIiwiZXhwIjo0MTAyNDQ0ODAwfQ.vhrE5RtXvt_YnbCrOlnrCurw2O820fEqWw6J_X51WpIOj_f0Mst6rxvy_t7xf8_DD7Vt_V4zCiX7JJxSCOz9Dg";

        let (header, _payload, signature) = split_token(token).unwrap();

        let tampered_payload = "eyJzdWIiOiJ1c2VyLTIiLCJ0eXBlIjoiQUNDRVNTIiwiZXhwIjo0MTAyNDQ0ODAwfQ";

        let result = verify_signature_parts(
            "ES256",
            header,
            tampered_payload,
            signature,
            "test-app-secret",
            Some(public_pem),
        );

        assert!(result.is_err());
    }
}
