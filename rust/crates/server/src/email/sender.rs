use crate::email::config::{EmailDriver, EmailSendConfig};
use crate::email::smtp::{SendMailOptions, SmtpError};

pub const EMAIL_SEND_RETRY_LIMIT: u32 = 3;
pub const EMAIL_SEND_RETRY_BACKOFF_STRATEGY: &str = "exponential";
pub const EMAIL_SEND_RETRY_BACKOFF_INITIAL_DELAY_MILLISECONDS: u64 = 5_000;
pub const EMAIL_SEND_RETRY_BACKOFF_JITTER: f64 = 0.5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmailSendError {
    MissingSmtpOptions,
    Smtp(SmtpError),
}

impl std::fmt::Display for EmailSendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSmtpOptions => {
                write!(f, "SMTP driver requires host and port to be defined")
            }
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

            super::smtp::send(options, message).await
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
