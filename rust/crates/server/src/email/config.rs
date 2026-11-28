use std::env;

pub const EMAIL_SMTP_PORT_DEFAULT: u16 = 587;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailDriver {
    Logger,
    Smtp,
}

impl EmailDriver {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_uppercase().as_str() {
            "LOGGER" => Some(Self::Logger),
            "SMTP" => Some(Self::Smtp),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Logger => "LOGGER",
            Self::Smtp => "SMTP",
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct SmtpAuth {
    pub user: String,
    pub pass: String,
}

impl std::fmt::Debug for SmtpAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SmtpAuth")
            .field("user", &"[redacted]")
            .field("pass", &"[redacted]")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmtpTransportOptions {
    pub host: String,
    pub port: u16,
    pub auth: Option<SmtpAuth>,
    pub secure: bool,
    pub ignore_tls: bool,
    pub require_tls: bool,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmailConfigError {
    MissingHostAndPort,
    InvalidDriver(String),
}

impl std::fmt::Display for EmailConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingHostAndPort => {
                write!(f, "SMTP driver requires host and port to be defined")
            }
            Self::InvalidDriver(driver) => write!(f, "Invalid email driver: {driver}"),
        }
    }
}

impl std::error::Error for EmailConfigError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailSendConfig {
    pub driver: EmailDriver,
    pub from_name: String,
    pub from_address: String,
    pub smtp: Option<SmtpTransportOptions>,
}

fn non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn truthy(value: Option<String>) -> bool {
    matches!(
        value
            .map(|value| value.trim().to_ascii_lowercase())
            .as_deref(),
        Some("true") | Some("1")
    )
}

fn hostname() -> Option<String> {
    let mut buffer = [0i8; 256];

    if unsafe { libc::gethostname(buffer.as_mut_ptr(), buffer.len()) } != 0 {
        return None;
    }

    let bytes: Vec<u8> = buffer
        .iter()
        .take_while(|byte| **byte != 0)
        .map(|byte| *byte as u8)
        .collect();

    String::from_utf8(bytes)
        .ok()
        .filter(|value| !value.is_empty())
}

impl SmtpTransportOptions {
    pub fn from_getter<F>(get: &F) -> Result<Self, EmailConfigError>
    where
        F: Fn(&str) -> Option<String>,
    {
        let host = non_empty(get("EMAIL_SMTP_HOST")).ok_or(EmailConfigError::MissingHostAndPort)?;
        let port = match non_empty(get("EMAIL_SMTP_PORT")) {
            Some(raw) => raw.parse::<u16>().ok().filter(|port| *port != 0),
            None => Some(EMAIL_SMTP_PORT_DEFAULT),
        }
        .ok_or(EmailConfigError::MissingHostAndPort)?;
        let user = non_empty(get("EMAIL_SMTP_USER"));
        let pass = non_empty(get("EMAIL_SMTP_PASSWORD"));
        let no_tls = truthy(get("EMAIL_SMTP_NO_TLS"));

        Ok(Self::build(
            host,
            port,
            user.zip(pass),
            no_tls,
            non_empty(get("EMAIL_SMTP_NAME")).or_else(hostname),
        ))
    }

    fn build(
        host: String,
        port: u16,
        auth: Option<(String, String)>,
        no_tls: bool,
        name: Option<String>,
    ) -> Self {
        let mut options = Self {
            host,
            port,
            auth: auth.map(|(user, pass)| SmtpAuth { user, pass }),
            secure: port == 465,
            ignore_tls: false,
            require_tls: false,
            name,
        };

        if no_tls {
            options.secure = false;
            options.ignore_tls = true;
        }

        options
    }

    pub fn from_env() -> Result<Self, EmailConfigError> {
        Self::from_getter(&|name| env::var(name).ok())
    }
}

impl EmailSendConfig {
    pub fn from_getter<F>(get: &F) -> Result<Self, EmailConfigError>
    where
        F: Fn(&str) -> Option<String>,
    {
        let driver_raw = non_empty(get("EMAIL_DRIVER")).unwrap_or_else(|| "LOGGER".to_string());
        let driver = EmailDriver::parse(&driver_raw)
            .ok_or_else(|| EmailConfigError::InvalidDriver(driver_raw.to_ascii_uppercase()))?;

        let smtp = match driver {
            EmailDriver::Logger => None,
            EmailDriver::Smtp => Some(SmtpTransportOptions::from_getter(get)?),
        };

        Ok(Self {
            driver,
            from_name: non_empty(get("EMAIL_FROM_NAME"))
                .unwrap_or_else(|| "Felix from Twenty".to_string()),
            from_address: non_empty(get("EMAIL_FROM_ADDRESS"))
                .unwrap_or_else(|| "noreply@yourdomain.com".to_string()),
            smtp,
        })
    }

    pub fn from_env() -> Result<Self, EmailConfigError> {
        Self::from_getter(&|name| env::var(name).ok())
    }

    pub fn logger_default() -> Self {
        Self {
            driver: EmailDriver::Logger,
            from_name: "Felix from Twenty".to_string(),
            from_address: "noreply@yourdomain.com".to_string(),
            smtp: None,
        }
    }

    pub fn build_from_header(&self) -> String {
        format!("{} <{}>", self.from_name, self.from_address)
    }

    pub fn build_invitation_from_header(&self, sender_name: &str) -> String {
        format!("{sender_name} (via Nudge) <{}>", self.from_address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn getter(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();

        move |name: &str| map.get(name).cloned()
    }

    #[test]
    fn driver_parses_upstream_enum_values() {
        assert_eq!(EmailDriver::parse("LOGGER"), Some(EmailDriver::Logger));
        assert_eq!(EmailDriver::parse("SMTP"), Some(EmailDriver::Smtp));
        assert_eq!(EmailDriver::parse("SES"), None);
    }

    #[test]
    fn driver_values_are_cast_to_upper_snake_case() {
        assert_eq!(EmailDriver::parse("smtp"), Some(EmailDriver::Smtp));
        assert_eq!(EmailDriver::parse("Logger"), Some(EmailDriver::Logger));
        assert_eq!(EmailDriver::Smtp.as_str(), "SMTP");
    }

    #[test]
    fn logger_is_the_default_driver() {
        let config = EmailSendConfig::from_getter(&getter(&[])).unwrap();

        assert_eq!(config.driver, EmailDriver::Logger);
        assert_eq!(config.from_name, "Felix from Twenty");
        assert_eq!(config.from_address, "noreply@yourdomain.com");
        assert_eq!(config.smtp, None);
    }

    #[test]
    fn invalid_driver_is_rejected() {
        let error = EmailSendConfig::from_getter(&getter(&[("EMAIL_DRIVER", "SES")])).unwrap_err();

        assert_eq!(error, EmailConfigError::InvalidDriver("SES".to_string()));
        assert_eq!(error.to_string(), "Invalid email driver: SES");
    }

    #[test]
    fn smtp_requires_host() {
        let error = EmailSendConfig::from_getter(&getter(&[("EMAIL_DRIVER", "SMTP")])).unwrap_err();

        assert_eq!(error, EmailConfigError::MissingHostAndPort);
        assert_eq!(
            error.to_string(),
            "SMTP driver requires host and port to be defined"
        );
    }

    #[test]
    fn smtp_auth_only_when_user_and_password_present() {
        let options = SmtpTransportOptions::from_getter(&getter(&[
            ("EMAIL_SMTP_HOST", "smtp.example.com"),
            ("EMAIL_SMTP_USER", "tester"),
        ]))
        .unwrap();

        assert_eq!(options.host, "smtp.example.com");
        assert_eq!(options.port, 587);
        assert_eq!(options.auth, None);
        assert!(!options.secure);

        let options = SmtpTransportOptions::from_getter(&getter(&[
            ("EMAIL_SMTP_HOST", "smtp.example.com"),
            ("EMAIL_SMTP_PORT", "465"),
            ("EMAIL_SMTP_USER", "tester"),
            ("EMAIL_SMTP_PASSWORD", "unused-test-secret"),
        ]))
        .unwrap();

        assert!(options.secure);
        assert!(options.auth.is_some());
    }

    #[test]
    fn no_tls_disables_secure_and_starttls() {
        let options = SmtpTransportOptions::from_getter(&getter(&[
            ("EMAIL_SMTP_HOST", "smtp.example.com"),
            ("EMAIL_SMTP_PORT", "25"),
            ("EMAIL_SMTP_NO_TLS", "true"),
        ]))
        .unwrap();

        assert!(!options.secure);
        assert!(options.ignore_tls);
        assert!(!options.require_tls);
    }

    #[test]
    fn invitation_from_header_carries_the_sender_and_the_shared_address() {
        let config = EmailSendConfig {
            driver: EmailDriver::Logger,
            from_name: "Acme Mailer".to_string(),
            from_address: "bot@acme.example".to_string(),
            smtp: None,
        };

        assert_eq!(
            config.build_invitation_from_header("Ada Lovelace"),
            "Ada Lovelace (via Nudge) <bot@acme.example>"
        );
    }

    #[test]
    fn build_from_header_follows_upstream_template() {
        let config = EmailSendConfig::from_getter(&getter(&[
            ("EMAIL_FROM_NAME", "Acme"),
            ("EMAIL_FROM_ADDRESS", "bot@acme.example"),
        ]))
        .unwrap();

        assert_eq!(config.build_from_header(), "Acme <bot@acme.example>");
    }

    #[test]
    fn values_are_trimmed_before_use() {
        let config = EmailSendConfig::from_getter(&getter(&[
            ("EMAIL_DRIVER", "  SMTP  "),
            ("EMAIL_SMTP_HOST", "  smtp.example.com "),
            ("EMAIL_SMTP_PORT", " 2525 "),
            ("EMAIL_FROM_ADDRESS", " bot@acme.example "),
        ]))
        .unwrap();

        assert_eq!(config.driver, EmailDriver::Smtp);
        assert_eq!(config.smtp.as_ref().unwrap().host, "smtp.example.com");
        assert_eq!(config.smtp.as_ref().unwrap().port, 2525);
        assert_eq!(config.from_address, "bot@acme.example");
    }

    #[test]
    fn no_tls_accepts_the_repository_boolean_spellings() {
        for raw in ["true", "TRUE", "1"] {
            let options = SmtpTransportOptions::from_getter(&getter(&[
                ("EMAIL_SMTP_HOST", "smtp.example.com"),
                ("EMAIL_SMTP_NO_TLS", raw),
            ]))
            .unwrap();

            assert!(options.ignore_tls, "{raw} should disable tls");
            assert!(!options.secure);
        }

        for raw in ["false", "0", "yes"] {
            let options = SmtpTransportOptions::from_getter(&getter(&[
                ("EMAIL_SMTP_HOST", "smtp.example.com"),
                ("EMAIL_SMTP_NO_TLS", raw),
            ]))
            .unwrap();

            assert!(!options.ignore_tls, "{raw} should keep tls enabled");
        }
    }

    #[test]
    fn invalid_port_falls_back_to_the_missing_host_error() {
        let error = SmtpTransportOptions::from_getter(&getter(&[
            ("EMAIL_SMTP_HOST", "smtp.example.com"),
            ("EMAIL_SMTP_PORT", "0"),
        ]))
        .unwrap_err();

        assert_eq!(error, EmailConfigError::MissingHostAndPort);
    }

    #[test]
    fn logger_default_matches_the_upstream_configuration() {
        let config = EmailSendConfig::logger_default();

        assert_eq!(config.driver, EmailDriver::Logger);
        assert_eq!(config, EmailSendConfig::from_getter(&getter(&[])).unwrap());
    }

    #[test]
    fn debug_output_never_carries_the_credentials() {
        let options = SmtpTransportOptions::from_getter(&getter(&[
            ("EMAIL_SMTP_HOST", "smtp.example.com"),
            ("EMAIL_SMTP_USER", "someone@acme.example"),
            ("EMAIL_SMTP_PASSWORD", "a-value-that-must-not-be-rendered"),
        ]))
        .unwrap();
        let rendered = format!("{options:?}");

        assert!(rendered.contains("[redacted]"));
        assert!(!rendered.contains("someone@acme.example"));
        assert!(!rendered.contains("a-value-that-must-not-be-rendered"));
    }
}
