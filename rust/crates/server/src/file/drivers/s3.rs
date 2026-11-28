use hmac::Hmac;
use hmac::Mac;
use sha2::Digest;
use sha2::Sha256;

const ALGORITHM: &str = "AWS4-HMAC-SHA256";
const UNSIGNED_PAYLOAD: &str = "UNSIGNED-PAYLOAD";

#[derive(Debug, Clone)]
pub struct S3Credentials {
    pub access_key_id: String,
    pub secret_access_key: String,
}

#[derive(Debug, Clone)]
pub struct S3Driver {
    pub bucket: String,
    pub region: String,
    pub endpoint: Option<String>,
    pub presign_base: Option<String>,
    pub credentials: Option<S3Credentials>,
}

impl S3Driver {
    pub fn host(&self) -> String {
        match &self.endpoint {
            Some(endpoint) => authority(endpoint),
            None => format!("{}.s3.{}.amazonaws.com", self.bucket, self.region),
        }
    }

    pub fn object_url(&self, object_key: &str) -> String {
        match &self.endpoint {
            Some(endpoint) => format!(
                "{}/{}/{}",
                endpoint.trim_end_matches('/'),
                self.bucket,
                aws_encode_path(object_key)
            ),
            None => format!(
                "https://{}.s3.{}.amazonaws.com/{}",
                self.bucket,
                self.region,
                aws_encode_path(object_key)
            ),
        }
    }

    pub fn presigned_upload_url(
        &self,
        object_key: &str,
        content_type: &str,
        content_length: u64,
        expires_in_secs: u64,
    ) -> Result<String, String> {
        let credentials = self
            .credentials
            .as_ref()
            .ok_or_else(|| String::from("s3 credentials are not configured"))?;
        let date_time = amz_date();
        let credential_scope = format!(
            "{}/{}/s3/aws4_request",
            &date_time[..8],
            self.region
        );
        let canonical_headers = format!(
            "content-length:{content_length}\ncontent-type:{content_type}\nhost:{}\n",
            self.host()
        );
        let signed_headers = "content-length;content-type;host";
        let canonical_query = [
            (
                "X-Amz-Algorithm".to_string(),
                ALGORITHM.to_string(),
                false,
            ),
            (
                "X-Amz-Credential".to_string(),
                format!("{}/{}", credentials.access_key_id, credential_scope),
                true,
            ),
            ("X-Amz-Date".to_string(), date_time.clone(), false),
            (
                "X-Amz-Expires".to_string(),
                expires_in_secs.to_string(),
                false,
            ),
            (
                "X-Amz-SignedHeaders".to_string(),
                signed_headers.to_string(),
                true,
            ),
        ]
        .into_iter()
        .map(|(name, value, encode_name)| {
            format!(
                "{}={}",
                if encode_name {
                    aws_encode(&name)
                } else {
                    name
                },
                aws_encode(&value)
            )
        })
        .collect::<Vec<_>>()
        .join("&");
        let canonical_request = format!(
            "PUT\n{}\n{canonical_query}\n{canonical_headers}\n{signed_headers}\n{UNSIGNED_PAYLOAD}",
            aws_encode_path(object_key)
        );
        let string_to_sign = format!(
            "{ALGORITHM}\n{date_time}\n{credential_scope}\n{}",
            hex_sha256(canonical_request.as_bytes())
        );
        let signature = hex(&hmac(
            &signing_key(&credentials.secret_access_key, &date_time[..8], &self.region),
            string_to_sign.as_bytes(),
        ));

        Ok(format!(
            "{}/{}?{canonical_query}&X-Amz-Signature={signature}",
            self.base_url().trim_end_matches('/'),
            aws_encode_path(object_key)
        ))
    }

    fn base_url(&self) -> String {
        match (&self.presign_base, &self.endpoint) {
            (Some(base), _) => base.clone(),
            (None, Some(endpoint)) => endpoint.clone(),
            (None, None) => format!(
                "https://{}.s3.{}.amazonaws.com",
                self.bucket, self.region
            ),
        }
    }
}

fn authority(endpoint: &str) -> String {
    endpoint
        .split_once("://")
        .map(|(_, rest)| rest.to_string())
        .unwrap_or_else(|| endpoint.to_string())
        .trim_end_matches('/')
        .to_string()
}

fn amz_date() -> String {
    chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
}

fn signing_key(secret: &str, date: &str, region: &str) -> [u8; 32] {
    let k_date = hmac(format!("AWS4{secret}").as_bytes(), date.as_bytes());
    let k_region = hmac(&k_date, region.as_bytes());
    let k_service = hmac(&k_region, b"s3");

    hmac(&k_service, b"aws4_request")
}

fn hmac(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(key).expect("hmac accepts any key length");
    mac.update(message);
    let digest = mac.finalize().into_bytes();
    let mut out = [0_u8; 32];

    out.copy_from_slice(&digest);
    out
}

fn hex_sha256(message: &[u8]) -> String {
    let mut hasher = Sha256::new();

    hasher.update(message);

    hex(&hasher.finalize())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn aws_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());

    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*byte as char);
            }
            other => encoded.push_str(&format!("%{other:02X}")),
        }
    }

    encoded
}

pub fn aws_encode_path(path: &str) -> String {
    path.split('/')
        .map(aws_encode)
        .collect::<Vec<_>>()
        .join("/")
}
