use std::time::Duration;

use crate::email::config::{EmailDriver, EmailSendConfig};
use crate::email::smtp::{SendMailOptions, SmtpError};

pub const EMAIL_SEND_RETRY_LIMIT: u32 = 3;
pub const EMAIL_SEND_RETRY_BACKOFF_STRATEGY: &str = "exponential";
pub const EMAIL_SEND_RETRY_BACKOFF_INITIAL_DELAY_MILLISECONDS: u64 = 5_000;
pub const EMAIL_SEND_RETRY_BACKOFF_JITTER: f64 = 0.5;
pub const EMAIL_SEND_ATTEMPTS: u32 = 1 + EMAIL_SEND_RETRY_LIMIT;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmailSendError {
    MissingSmtpOptions,
    Config(String),
    Smtp(SmtpError),
}

impl std::fmt::Display for EmailSendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSmtpOptions => {
                write!(f, "SMTP driver requires host and port to be defined")
            }
            Self::Config(message) => write!(f, "{message}"),
            Self::Smtp(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for EmailSendError {}

impl From<SmtpError> for EmailSendError {
    fn from(error: SmtpError) -> Self {
        Self::Smtp(error)
    }
}

pub fn format_logger_driver_output(message: &SendMailOptions) -> String {
    format!(
        "Sent email to: {}\nFrom: {}\nSubject: {}\nContent Text: {}\nContent HTML: {}",
        message.to.join(","),
        message.from,
        message.subject,
        message.text,
        message.html
    )
}

pub fn backoff_delay_millis(
    attempt: u32,
    initial_delay_millis: u64,
    jitter: f64,
    random: f64,
) -> u64 {
    let base =
        initial_delay_millis.saturating_mul(1u64.checked_shl(attempt).unwrap_or(u64::MAX));
    let factor = 1.0 + jitter.clamp(0.0, 1.0) * random.clamp(0.0, 1.0);

    (base as f64 * factor).round() as u64
}

pub fn upstream_backoff_delay(attempt: u32) -> Duration {
    use rand::Rng;

    let random = rand::thread_rng().gen_range(0.0..1.0);

    Duration::from_millis(backoff_delay_millis(
        attempt,
        EMAIL_SEND_RETRY_BACKOFF_INITIAL_DELAY_MILLISECONDS,
        EMAIL_SEND_RETRY_BACKOFF_JITTER,
        random,
    ))
}

pub async fn send_with_retry(
    config: &EmailSendConfig,
    message: &SendMailOptions,
    delay_for_attempt: impl Fn(u32) -> Duration,
) -> Result<(), EmailSendError> {
    let mut last_error = EmailSendError::MissingSmtpOptions;

    for attempt in 0..EMAIL_SEND_ATTEMPTS {
        match send(config, message).await {
            Ok(()) => return Ok(()),
            Err(error) => last_error = error,
        }

        if attempt + 1 < EMAIL_SEND_ATTEMPTS {
            tokio::time::sleep(delay_for_attempt(attempt)).await;
        }
    }

    Err(last_error)
}

pub async fn send(
    config: &EmailSendConfig,
    message: &SendMailOptions,
) -> Result<(), EmailSendError> {
    match config.driver {
        EmailDriver::Logger => {
            tracing::info!(
                target: "LoggerDriver",
                "{}",
                format_logger_driver_output(message)
            );

            Ok(())
        }
        EmailDriver::Smtp => {
            let options = config
                .smtp
                .as_ref()
                .ok_or(EmailSendError::MissingSmtpOptions)?;

            super::smtp::send(options, message)
                .await
                .map_err(EmailSendError::from)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::email::config::SmtpTransportOptions;

    fn message() -> SendMailOptions {
        SendMailOptions {
            from: "Felix from Twenty <noreply@yourdomain.com>".to_string(),
            to: vec![
                "one@acme.example".to_string(),
                "two@acme.example".to_string(),
            ],
            subject: "Verify your email".to_string(),
            text: "Click here".to_string(),
            html: "<p>Click here</p>".to_string(),
        }
    }

    #[test]
    fn logger_output_matches_upstream_template() {
        assert_eq!(
            format_logger_driver_output(&message()),
            "Sent email to: one@acme.example,two@acme.example\n\
             From: Felix from Twenty <noreply@yourdomain.com>\n\
             Subject: Verify your email\n\
             Content Text: Click here\n\
             Content HTML: <p>Click here</p>"
        );
    }

    #[tokio::test]
    async fn logger_driver_succeeds_without_transport() {
        let config = EmailSendConfig::from_getter(&|_| None).unwrap();

        send(&config, &message()).await.unwrap();
    }

    #[tokio::test]
    async fn smtp_driver_delivers_over_local_listener() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = tokio::io::BufStream::new(&mut socket);
            use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _};

            buf.write_all(b"220 test ESMTP\r\n").await.unwrap();
            buf.flush().await.unwrap();

            let mut received = String::new();

            loop {
                let mut line = String::new();
                buf.read_line(&mut line).await.unwrap();
                received.push_str(&line);

                let reply: &[u8] = if line.starts_with("EHLO") {
                    b"250-test\r\n"
                } else if line.starts_with("MAIL") || line.starts_with("RCPT") {
                    b"250 ok\r\n"
                } else if line.starts_with("DATA") {
                    b"354 go ahead\r\n"
                } else if line == ".\r\n" {
                    b"250 accepted\r\n"
                } else if line.starts_with("QUIT") {
                    b"221 bye\r\n"
                } else {
                    b"250 ok\r\n"
                };

                buf.write_all(reply).await.unwrap();
                buf.flush().await.unwrap();

                if line.starts_with("QUIT") {
                    break;
                }
            }

            received
        });

        let config = EmailSendConfig {
            driver: EmailDriver::Smtp,
            from_name: "Felix from Twenty".to_string(),
            from_address: "noreply@yourdomain.com".to_string(),
            smtp: Some(SmtpTransportOptions {
                host: addr.ip().to_string(),
                port: addr.port(),
                auth: None,
                secure: false,
                ignore_tls: true,
                require_tls: false,
                name: Some("nudge.test".to_string()),
            }),
        };

        send(&config, &message()).await.unwrap();

        let received = server.await.unwrap();
        assert!(received.contains("Subject: Verify your email"));
        assert!(received.contains("<p>Click here</p>"));
    }

    #[test]
    fn retry_backoff_matches_upstream_constants() {
        assert_eq!(EMAIL_SEND_RETRY_LIMIT, 3);
        assert_eq!(EMAIL_SEND_RETRY_BACKOFF_STRATEGY, "exponential");
        assert_eq!(EMAIL_SEND_RETRY_BACKOFF_INITIAL_DELAY_MILLISECONDS, 5_000);
        assert_eq!(EMAIL_SEND_RETRY_BACKOFF_JITTER, 0.5);
        assert_eq!(EMAIL_SEND_ATTEMPTS, 4);
    }

    #[test]
    fn exponential_backoff_doubles_and_applies_jitter() {
        assert_eq!(backoff_delay_millis(0, 5_000, 0.5, 0.0), 5_000);
        assert_eq!(backoff_delay_millis(1, 5_000, 0.5, 0.0), 10_000);
        assert_eq!(backoff_delay_millis(2, 5_000, 0.5, 0.0), 20_000);
        assert_eq!(backoff_delay_millis(0, 5_000, 0.5, 1.0), 7_500);
        assert_eq!(backoff_delay_millis(1, 5_000, 0.0, 1.0), 10_000);
        assert_eq!(backoff_delay_millis(3, 5_000, 0.5, 0.5), 37_500);
    }

    #[test]
    fn upstream_backoff_stays_within_the_jitter_band() {
        for attempt in 0..3 {
            let delay = upstream_backoff_delay(attempt).as_millis() as u64;
            let base = 5_000u64 << attempt;

            assert!(
                (base..=base + base / 2).contains(&delay),
                "attempt {attempt} produced {delay}ms outside the exponential band"
            );
        }
    }

    #[tokio::test]
    async fn retry_gives_up_after_the_upstream_attempt_budget() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let accepted = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter = std::sync::Arc::clone(&accepted);

        let server = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((socket, _)) => {
                        counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        drop(socket);
                    }
                    Err(_) => break,
                }
            }
        });

        let config = EmailSendConfig {
            driver: EmailDriver::Smtp,
            from_name: "Felix from Twenty".to_string(),
            from_address: "noreply@yourdomain.com".to_string(),
            smtp: Some(SmtpTransportOptions {
                host: addr.ip().to_string(),
                port: addr.port(),
                auth: None,
                secure: false,
                ignore_tls: true,
                require_tls: false,
                name: Some("nudge.test".to_string()),
            }),
        };

        let result = send_with_retry(&config, &message(), |_| Duration::ZERO).await;

        assert!(matches!(result, Err(EmailSendError::Smtp(_))));
        assert_eq!(
            accepted.load(std::sync::atomic::Ordering::SeqCst),
            EMAIL_SEND_ATTEMPTS as usize
        );

        server.abort();
    }

    #[tokio::test]
    async fn retry_stops_at_the_first_success() {
        let config = EmailSendConfig::from_getter(&|_| None).unwrap();

        send_with_retry(&config, &message(), |_| Duration::ZERO)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn renders_upstream_template_and_delivers_over_local_smtp() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = tokio::io::BufStream::new(&mut socket);
            use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _};

            buf.write_all(b"220 test ESMTP\r\n").await.unwrap();
            buf.flush().await.unwrap();

            let mut received = String::new();

            loop {
                let mut line = String::new();
                buf.read_line(&mut line).await.unwrap();
                received.push_str(&line);

                let reply: &[u8] = if line.starts_with("EHLO") {
                    b"250-test\r\n"
                } else if line.starts_with("MAIL") || line.starts_with("RCPT") {
                    b"250 ok\r\n"
                } else if line.starts_with("DATA") {
                    b"354 go ahead\r\n"
                } else if line == ".\r\n" {
                    b"250 accepted\r\n"
                } else if line.starts_with("QUIT") {
                    b"221 bye\r\n"
                } else {
                    b"250 ok\r\n"
                };

                buf.write_all(reply).await.unwrap();
                buf.flush().await.unwrap();

                if line.starts_with("QUIT") {
                    break;
                }
            }

            received
        });

        let config = EmailSendConfig::from_getter(&|name: &str| match name {
            "EMAIL_DRIVER" => Some("SMTP".to_string()),
            "EMAIL_SMTP_HOST" => Some(addr.ip().to_string()),
            "EMAIL_SMTP_PORT" => Some(addr.port().to_string()),
            "EMAIL_SMTP_NO_TLS" => Some("true".to_string()),
            _ => None,
        })
        .unwrap();

        let rendered = crate::email::templates::send_email_verification_link_email(
            "https://app.twenty.com/verify-email?token=abc123",
            "en",
            false,
        );
        let message = SendMailOptions {
            from: config.from_header(),
            to: vec!["recipient@target.test".to_string()],
            subject: "Confirm your email address".to_string(),
            text: rendered.text,
            html: rendered.html,
        };

        send(&config, &message).await.unwrap();

        let received = server.await.unwrap();
        assert!(received.contains("Subject: Confirm your email address"));
        assert!(received.contains("Felix from Twenty <noreply@yourdomain.com>"));
        assert!(received.contains(">Confirm your email address<"));
        assert!(received.contains("token=3Dabc123"));
    }
}
