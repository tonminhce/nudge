use std::sync::Arc;
use std::time::{Duration, Instant};

use p256::ecdsa::SigningKey;
use p256::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
use p256::SecretKey;
use rand::rngs::OsRng;
use rand::RngCore;
use sqlx::PgPool;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::crypto;
use super::error::{AuthError, AuthResult};

const CURRENT_KEY_CACHE_TTL: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct SigningKeyPair {
    pub id: Uuid,
    pub public_key_pem: String,
    pub private_key_pem: String,
}

pub struct SigningKeyManager {
    pool: PgPool,
    raw_keys: Vec<String>,
    cache: RwLock<Option<(Instant, Arc<SigningKeyPair>)>>,
}

impl SigningKeyManager {
    pub fn new(pool: PgPool, raw_keys: Vec<String>) -> Self {
        Self {
            pool,
            raw_keys,
            cache: RwLock::new(None),
        }
    }

    pub async fn current_key(&self) -> AuthResult<Arc<SigningKeyPair>> {
        {
            let cache = self.cache.read().await;

            if let Some((loaded_at, key)) = cache.as_ref() {
                if loaded_at.elapsed() < CURRENT_KEY_CACHE_TTL {
                    return Ok(key.clone());
                }
            }
        }

        let mut cache = self.cache.write().await;

        if let Some((loaded_at, key)) = cache.as_ref() {
            if loaded_at.elapsed() < CURRENT_KEY_CACHE_TTL {
                return Ok(key.clone());
            }
        }

        let key = Arc::new(self.load_or_create_current_key().await?);
        *cache = Some((Instant::now(), key.clone()));

        Ok(key)
    }

    pub async fn public_key_for_kid(&self, kid: &Uuid) -> AuthResult<String> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT \"publicKey\" FROM core.\"signingKey\" WHERE id = $1 AND \"revokedAt\" IS NULL",
        )
        .bind(kid)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|(public_key_pem,)| public_key_pem)
            .ok_or_else(|| AuthError::unauthenticated("Token invalid."))
    }

    async fn load_or_create_current_key(&self) -> AuthResult<SigningKeyPair> {
        let row: Option<(Uuid, String, Option<String>)> = sqlx::query_as(
            "SELECT id, \"publicKey\", \"privateKey\" \
             FROM core.\"signingKey\" \
             WHERE \"isCurrent\" = true AND \"revokedAt\" IS NULL \
             ORDER BY \"createdAt\" DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some((id, public_key_pem, Some(encrypted_private))) = row {
            let private_key_pem = crypto::decrypt_versioned(&self.raw_keys, &encrypted_private)
                .map_err(AuthError::internal)?;

            return Ok(SigningKeyPair {
                id,
                public_key_pem,
                private_key_pem,
            });
        }

        self.create_current_key().await
    }

    async fn create_current_key(&self) -> AuthResult<SigningKeyPair> {
        let mut secret_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut secret_bytes);

        let secret = SecretKey::from_slice(&secret_bytes)
            .map_err(|error| AuthError::internal(error.to_string()))?;
        let signing_key = SigningKey::from(&secret);

        let private_key_pem = signing_key
            .to_pkcs8_pem(LineEnding::LF)
            .map_err(|error| AuthError::internal(error.to_string()))?
            .to_string();

        let public_key = signing_key.verifying_key();
        let public_key_pem = public_key
            .to_public_key_pem(LineEnding::LF)
            .map_err(|error| AuthError::internal(error.to_string()))?;

        let primary_key = self
            .raw_keys
            .first()
            .ok_or_else(|| AuthError::internal("no encryption key is configured"))?;

        let encrypted_private = crypto::encrypt_versioned(primary_key, &private_key_pem);

        let mut transaction = self.pool.begin().await?;

        sqlx::query(
            "UPDATE core.\"signingKey\" SET \"isCurrent\" = false, \"updatedAt\" = now() \
             WHERE \"isCurrent\" = true",
        )
        .execute(&mut *transaction)
        .await?;

        let id: (Uuid,) = sqlx::query_as(
            "INSERT INTO core.\"signingKey\" \
             (\"publicKey\", \"privateKey\", \"isCurrent\") \
             VALUES ($1, $2, true) RETURNING id",
        )
        .bind(&public_key_pem)
        .bind(&encrypted_private)
        .fetch_one(&mut *transaction)
        .await?;

        transaction.commit().await?;

        Ok(SigningKeyPair {
            id: id.0,
            public_key_pem,
            private_key_pem,
        })
    }
}
