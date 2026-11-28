use std::sync::OnceLock;

use crate::email::config::{EmailConfigError, EmailDriver, EmailSendConfig};
use crate::email::sender::{send_with_retry, upstream_backoff_delay, EmailSendError};
use crate::email::smtp::SendMailOptions;
use crate::email::templates::RenderedEmail;

static EMAIL_CONFIG: OnceLock<Result<EmailSendConfig, EmailConfigError>> = OnceLock::new();

pub fn init(config: EmailSendConfig) {
    let _ = EMAIL_CONFIG.set(Ok(config));
}

pub fn init_from_env() -> Result<EmailSendConfig, EmailConfigError> {
    let config = EmailSendConfig::from_env();

    let _ = EMAIL_CONFIG.set(config.clone());

    match &config {
        Ok(config) => tracing::info!(
            driver = config.driver.as_str(),
            from_address = %config.from_address,
            smtp_host = config
                .smtp
                .as_ref()
                .map(|smtp| smtp.host.as_str())
                .unwrap_or("-"),
            "email driver configured"
        ),
        Err(error) => tracing::warn!(%error, "email configuration is incomplete"),
    }

    config
}

pub fn config() -> Option<&'static EmailSendConfig> {
    EMAIL_CONFIG.get()?.as_ref().ok()
}

pub fn build_message_with_from(
    config: &EmailSendConfig,
    from: Option<&str>,
    to: &str,
    subject: &str,
    rendered: &RenderedEmail,
) -> SendMailOptions {
    let default_from = config.build_from_header();

    SendMailOptions {
        from: from.unwrap_or(&default_from).to_string(),
        to: vec![to.to_string()],
        subject: subject.to_string(),
        text: rendered.text.clone(),
        html: rendered.html.clone(),
    }
}

pub fn build_message(
    config: &EmailSendConfig,
    to: &str,
    subject: &str,
    rendered: &RenderedEmail,
) -> SendMailOptions {
    build_message_with_from(config, None, to, subject, rendered)
}

pub async fn deliver_with_from(
    config: &EmailSendConfig,
    from: Option<&str>,
    to: &str,
    subject: &str,
    rendered: &RenderedEmail,
) -> Result<(), EmailSendError> {
    let message = build_message_with_from(config, from, to, subject, rendered);

    if config.driver == EmailDriver::Smtp && config.smtp.is_none() {
        return Err(EmailSendError::MissingSmtpOptions);
    }

    send_with_retry(config, &message, upstream_backoff_delay).await
}

pub async fn deliver(
    config: &EmailSendConfig,
    to: &str,
    subject: &str,
    rendered: &RenderedEmail,
) -> Result<(), EmailSendError> {
    deliver_with_from(config, None, to, subject, rendered).await
}

pub async fn send_transactional_email_with_from(
    from: Option<&str>,
    to: &str,
    subject: &str,
    rendered: &RenderedEmail,
) -> Result<(), EmailSendError> {
    match EMAIL_CONFIG.get() {
        Some(Ok(config)) => deliver_with_from(config, from, to, subject, rendered).await,
        Some(Err(error)) => Err(EmailSendError::Config(error.to_string())),
        None => Err(EmailSendError::Config(
            "email driver is not configured".to_string(),
        )),
    }
}

pub async fn send_transactional_email(
    to: &str,
    subject: &str,
    rendered: &RenderedEmail,
) -> Result<(), EmailSendError> {
    send_transactional_email_with_from(None, to, subject, rendered).await
}

pub fn queue_transactional_email_with_from(
    from: Option<&str>,
    to: &str,
    subject: &str,
    rendered: &RenderedEmail,
) -> bool {
    if tokio::runtime::Handle::try_current().is_err() {
        return false;
    }

    let from = from.map(str::to_string);
    let to = to.to_string();
    let subject = subject.to_string();
    let rendered = rendered.clone();

    tokio::spawn(async move {
        if let Err(error) =
            send_transactional_email_with_from(from.as_deref(), &to, &subject, &rendered).await
        {
            tracing::error!(%error, "sending the transactional email failed");
        }
    });

    true
}

pub fn queue_transactional_email(to: &str, subject: &str, rendered: &RenderedEmail) -> bool {
    queue_transactional_email_with_from(None, to, subject, rendered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::email::config::SmtpTransportOptions;
    use crate::email::smtp::spawn_fake_smtp_server;
    use crate::email::templates::{send_email_verification_link_email, send_invite_link_email};

    fn config() -> EmailSendConfig {
        EmailSendConfig {
            driver: EmailDriver::Logger,
            from_name: "Felix from Twenty".to_string(),
            from_address: "noreply@yourdomain.com".to_string(),
            smtp: None,
        }
    }

    fn rendered() -> RenderedEmail {
        send_email_verification_link_email("https://app/verify?token=1", "en", false)
    }

    #[test]
    fn an_explicit_from_header_replaces_the_configured_default() {
        let rendered = rendered();
        let message = build_message_with_from(
            &config(),
            Some("Ada Lovelace (via Nudge) <noreply@yourdomain.com>"),
            "user@target.test",
            "Join your team on Nudge",
            &rendered,
        );

        assert_eq!(
            message.from,
            "Ada Lovelace (via Nudge) <noreply@yourdomain.com>"
        );
    }

    #[test]
    fn message_uses_the_upstream_from_header_and_rendered_bodies() {
        let rendered = rendered();
        let message = build_message(&config(), "user@target.test", "Welcome", &rendered);

        assert_eq!(message.from, "Felix from Twenty <noreply@yourdomain.com>");
        assert_eq!(message.to, vec!["user@target.test".to_string()]);
        assert_eq!(message.subject, "Welcome");
        assert_eq!(message.html, rendered.html);
        assert_eq!(message.text, rendered.text);
    }

    #[tokio::test]
    async fn an_invitation_over_smtp_carries_the_sender_from_header() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server =
            spawn_fake_smtp_server(listener, b"250-test greets\r\n250 AUTH PLAIN LOGIN\r\n").await;

        let config = EmailSendConfig {
            driver: EmailDriver::Smtp,
            smtp: Some(SmtpTransportOptions {
                host: addr.ip().to_string(),
                port: addr.port(),
                auth: None,
                secure: false,
                ignore_tls: true,
                require_tls: false,
                name: Some("nudge.test".to_string()),
            }),
            ..config()
        };
        let rendered = send_invite_link_email(
            "https://app.test/invite?token=1",
            Some("Acme"),
            None,
            "boss@acme.test",
            "ada",
            "https://app.test",
            "en",
        );
        let from = config.build_invitation_from_header("Ada Lovelace");

        deliver_with_from(
            &config,
            Some(&from),
            "new@acme.test",
            "Join your team on Nudge",
            &rendered,
        )
        .await
        .unwrap();

        let transcript = server.await.unwrap();

        assert!(transcript.contains("MAIL FROM:<noreply@yourdomain.com>\r\n"));
        assert!(transcript.contains("RCPT TO:<new@acme.test>\r\n"));
        assert!(transcript.contains("From: Ada Lovelace (via Nudge) <noreply@yourdomain.com>"));
        assert!(transcript.contains("Subject: Join your team on Nudge"));
        assert!(
            transcript.contains("Accept invite https://app.test/invite?token=3D1"),
            "transcript: {transcript}"
        );
    }

    #[tokio::test]
    async fn logger_driver_path_needs_no_smtp_configuration() {
        deliver(&config(), "user@target.test", "Welcome", &rendered())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn smtp_driver_without_transport_options_reports_the_upstream_error() {
        let broken = EmailSendConfig {
            driver: EmailDriver::Smtp,
            ..config()
        };
        let error = deliver(&broken, "user@target.test", "Welcome", &rendered())
            .await
            .unwrap_err();

        assert_eq!(error, EmailSendError::MissingSmtpOptions);
        assert_eq!(
            error.to_string(),
            "SMTP driver requires host and port to be defined"
        );
    }

    #[tokio::test]
    async fn missing_global_driver_reports_without_touching_the_network() {
        if EMAIL_CONFIG.get().is_some() {
            return;
        }

        let error = send_transactional_email("user@target.test", "Welcome", &rendered())
            .await
            .expect_err("the process-wide driver is only installed by the server binary");

        assert!(matches!(error, EmailSendError::Config(_)));
    }
}
