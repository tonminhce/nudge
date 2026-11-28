use std::sync::Arc;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::{DateTime, Utc};
use rustls::pki_types::ServerName;
use tokio::io::{AsyncBufReadExt as _, AsyncRead, AsyncWrite, AsyncWriteExt as _, BufStream};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

use crate::email::config::SmtpTransportOptions;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmtpError {
    Io(String),
    Protocol(String),
    Tls(String),
    AuthUnsupported,
}

impl std::fmt::Display for SmtpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(message) => write!(f, "smtp transport error: {message}"),
            Self::Protocol(message) => write!(f, "smtp protocol error: {message}"),
            Self::Tls(message) => write!(f, "smtp tls error: {message}"),
            Self::AuthUnsupported => write!(f, "the smtp server does not support authentication"),
        }
    }
}

impl std::error::Error for SmtpError {}

impl From<std::io::Error> for SmtpError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendMailOptions {
    pub from: String,
    pub to: Vec<String>,
    pub subject: String,
    pub text: String,
    pub html: String,
}

enum Connection {
    Plain(TcpStream),
    Secure(Box<tokio_rustls::client::TlsStream<TcpStream>>),
}

type Buffered = BufStream<Connection>;

impl AsyncRead for Connection {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::Plain(stream) => std::pin::Pin::new(stream).poll_read(cx, buf),
            Self::Secure(stream) => std::pin::Pin::new(stream.as_mut()).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for Connection {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match self.get_mut() {
            Self::Plain(stream) => std::pin::Pin::new(stream).poll_write(cx, buf),
            Self::Secure(stream) => std::pin::Pin::new(stream.as_mut()).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::Plain(stream) => std::pin::Pin::new(stream).poll_flush(cx),
            Self::Secure(stream) => std::pin::Pin::new(stream.as_mut()).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::Plain(stream) => std::pin::Pin::new(stream).poll_shutdown(cx),
            Self::Secure(stream) => std::pin::Pin::new(stream.as_mut()).poll_shutdown(cx),
        }
    }
}

async fn read_reply(stream: &mut Buffered) -> Result<(u16, Vec<String>), SmtpError> {
    let mut lines = Vec::new();

    loop {
        let mut line = String::new();
        let read = stream.read_line(&mut line).await?;

        if read == 0 {
            return Err(SmtpError::Protocol(
                "the server closed the connection".into(),
            ));
        }

        let line = line.trim_end_matches(['\r', '\n']).to_string();

        if line.len() < 3 {
            return Err(SmtpError::Protocol(format!("malformed reply: {line}")));
        }

        let code: u16 = line[..3]
            .parse()
            .map_err(|_| SmtpError::Protocol(format!("malformed reply code: {}", &line[..3])))?;
        let last_line = line.get(3..4) != Some("-");
        lines.push(line);

        if last_line {
            return Ok((code, lines));
        }
    }
}

async fn expect(stream: &mut Buffered, codes: &[u16]) -> Result<Vec<String>, SmtpError> {
    let (code, lines) = read_reply(stream).await?;

    if !codes.contains(&code) {
        return Err(SmtpError::Protocol(format!(
            "unexpected reply {code}: {}",
            lines.last().cloned().unwrap_or_default()
        )));
    }

    Ok(lines)
}

async fn command(
    stream: &mut Buffered,
    line: &str,
    codes: &[u16],
) -> Result<Vec<String>, SmtpError> {
    stream.write_all(line.as_bytes()).await?;
    stream.write_all(b"\r\n").await?;
    stream.flush().await?;

    expect(stream, codes).await
}

fn tls_connector() -> Result<TlsConnector, SmtpError> {
    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    Ok(TlsConnector::from(Arc::new(config)))
}

fn server_name(host: &str) -> Result<ServerName<'static>, SmtpError> {
    ServerName::try_from(host.to_string()).map_err(|error| SmtpError::Tls(error.to_string()))
}

async fn ehlo(stream: &mut Buffered, name: &str) -> Result<Vec<String>, SmtpError> {
    command(stream, &format!("EHLO {name}"), &[250]).await
}

fn advertises(lines: &[String], feature: &str) -> bool {
    lines.iter().any(|line| {
        line.split_whitespace()
            .skip(1)
            .any(|token| token.eq_ignore_ascii_case(feature))
    })
}

fn encode_data(data: &str) -> String {
    data.lines()
        .map(|line| {
            if let Some(rest) = line.strip_prefix('.') {
                format!("..{rest}")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<String>>()
        .join("\r\n")
}

fn address_inner(address: &str) -> &str {
    address
        .split_once('<')
        .and_then(|(_, rest)| rest.split_once('>'))
        .map(|(inner, _)| inner)
        .unwrap_or(address)
        .trim()
}

fn encode_header_value(value: &str) -> String {
    if value.is_ascii() {
        return value.to_string();
    }

    format!("=?UTF-8?B?{}?=", BASE64.encode(value.as_bytes()))
}

pub fn encode_quoted_printable(value: &str) -> String {
    let mut out = String::new();

    for (index, line) in value.split('\n').enumerate() {
        if index > 0 {
            out.push_str("=\r\n");
        }

        let bytes = line.as_bytes();
        let mut line_len = 0;

        for (position, byte) in bytes.iter().enumerate() {
            let is_last = position + 1 == bytes.len();
            let token =
                if (0x20..=0x7e).contains(byte) && *byte != b'=' && !(is_last && *byte == b' ') {
                    (*byte as char).to_string()
                } else {
                    format!("={:02X}", byte)
                };

            if line_len + token.len() > 74 {
                out.push_str("=\r\n");
                line_len = 0;
            }

            out.push_str(&token);
            line_len += token.len();
        }
    }

    out
}

pub fn build_rfc822(message: &SendMailOptions, now: DateTime<Utc>) -> String {
    let boundary = format!("_NmP-{}", uuid::Uuid::now_v7().simple());
    let date = now.format("%a, %d %b %Y %H:%M:%S +0000").to_string();

    let mut out = String::new();
    out.push_str(&format!("Date: {date}\r\n"));
    out.push_str(&format!("From: {}\r\n", encode_header_value(&message.from)));
    out.push_str(&format!(
        "To: {}\r\n",
        encode_header_value(&message.to.join(", "))
    ));
    out.push_str(&format!(
        "Subject: {}\r\n",
        encode_header_value(&message.subject)
    ));
    out.push_str("MIME-Version: 1.0\r\n");
    out.push_str(&format!(
        "Content-Type: multipart/alternative;\r\n boundary=\"{boundary}\"\r\n"
    ));
    out.push_str("\r\nThis is a multi-part message in MIME format.\r\n");
    out.push_str(&format!("--{boundary}\r\n"));
    out.push_str("Content-Type: text/plain; charset=utf-8\r\n");
    out.push_str("Content-Transfer-Encoding: quoted-printable\r\n\r\n");
    out.push_str(&encode_quoted_printable(&message.text));
    out.push_str(&format!("\r\n--{boundary}\r\n"));
    out.push_str("Content-Type: text/html; charset=utf-8\r\n");
    out.push_str("Content-Transfer-Encoding: quoted-printable\r\n\r\n");
    out.push_str(&encode_quoted_printable(&message.html));
    out.push_str(&format!("\r\n--{boundary}--\r\n"));

    out
}

pub async fn send(
    options: &SmtpTransportOptions,
    message: &SendMailOptions,
) -> Result<(), SmtpError> {
    let socket = TcpStream::connect((options.host.as_str(), options.port)).await?;
    socket.set_nodelay(true).ok();

    let mut stream = if options.secure {
        let tls = tls_connector()?
            .connect(server_name(&options.host)?, socket)
            .await
            .map_err(|error| SmtpError::Tls(error.to_string()))?;
        Buffered::new(Connection::Secure(Box::new(tls)))
    } else {
        Buffered::new(Connection::Plain(socket))
    };

    expect(&mut stream, &[220]).await?;

    let name = options
        .name
        .clone()
        .unwrap_or_else(|| "localhost".to_string());
    let mut capabilities = ehlo(&mut stream, &name).await?;

    if !options.secure && !options.ignore_tls && advertises(&capabilities, "STARTTLS") {
        command(&mut stream, "STARTTLS", &[220]).await?;
        stream = upgrade(stream, &options.host).await?;
        capabilities = ehlo(&mut stream, &name).await?;
    }

    if let Some(auth) = &options.auth {
        if advertises(&capabilities, "AUTH") && advertises(&capabilities, "LOGIN") {
            command(&mut stream, "AUTH LOGIN", &[334]).await?;
            command(&mut stream, &BASE64.encode(&auth.user), &[334]).await?;
            command(&mut stream, &BASE64.encode(&auth.pass), &[235]).await?;
        } else if advertises(&capabilities, "AUTH") && advertises(&capabilities, "PLAIN") {
            let token = BASE64.encode(format!("\0{}\0{}", auth.user, auth.pass));
            command(&mut stream, &format!("AUTH PLAIN {token}"), &[235]).await?;
        } else {
            return Err(SmtpError::AuthUnsupported);
        }
    }

    command(
        &mut stream,
        &format!("MAIL FROM:<{}>", address_inner(&message.from)),
        &[250],
    )
    .await?;

    for recipient in &message.to {
        command(
            &mut stream,
            &format!("RCPT TO:<{}>", address_inner(recipient)),
            &[250, 251],
        )
        .await?;
    }

    command(&mut stream, "DATA", &[354]).await?;
    stream
        .write_all(encode_data(&build_rfc822(message, Utc::now())).as_bytes())
        .await?;
    stream.write_all(b"\r\n.\r\n").await?;
    stream.flush().await?;
    expect(&mut stream, &[250]).await?;

    let _ = command(&mut stream, "QUIT", &[221]).await;

    Ok(())
}

async fn upgrade(stream: Buffered, host: &str) -> Result<Buffered, SmtpError> {
    let connector = tls_connector()?;
    let inner = stream.into_inner();
    let plain = match inner {
        Connection::Plain(socket) => socket,
        Connection::Secure(_) => return Err(SmtpError::Tls("already tls".into())),
    };
    let tls = connector
        .connect(server_name(host)?, plain)
        .await
        .map_err(|error| SmtpError::Tls(error.to_string()))?;

    Ok(BufStream::new(Connection::Secure(Box::new(tls))))
}

#[cfg(test)]
pub(crate) async fn spawn_fake_smtp_server(
    listener: tokio::net::TcpListener,
    ehlo_reply: &'static [u8],
) -> tokio::task::JoinHandle<String> {
    tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut buf = Buffered::new(Connection::Plain(socket));
        let mut transcript = String::new();
        let mut auth_step = 0u8;

        buf.write_all(b"220 test ESMTP\r\n").await.unwrap();
        buf.flush().await.unwrap();

        loop {
            let mut line = String::new();
            if buf.read_line(&mut line).await.unwrap() == 0 {
                break;
            }
            transcript.push_str(&line);

            let reply: &[u8] = if line.starts_with("EHLO") {
                ehlo_reply
            } else if line == "AUTH LOGIN\r\n" {
                auth_step = 1;
                b"334 VXNlcm5hbWU6\r\n"
            } else if auth_step == 1 {
                auth_step = 2;
                b"334 UGFzc3dvcmQ6\r\n"
            } else if auth_step == 2 {
                auth_step = 0;
                b"235 2.7.0 Authentication successful\r\n"
            } else if line.starts_with("AUTH PLAIN ") {
                b"235 2.7.0 Authentication successful\r\n"
            } else if line.starts_with("MAIL") || line.starts_with("RCPT") {
                b"250 ok\r\n"
            } else if line.starts_with("DATA") {
                buf.write_all(b"354 go ahead\r\n").await.unwrap();
                buf.flush().await.unwrap();

                loop {
                    let mut data = String::new();
                    if buf.read_line(&mut data).await.unwrap() == 0 {
                        break;
                    }
                    transcript.push_str(&data);
                    if data == ".\r\n" {
                        break;
                    }
                }

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

        transcript
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(addr: std::net::SocketAddr) -> SmtpTransportOptions {
        SmtpTransportOptions {
            host: addr.ip().to_string(),
            port: addr.port(),
            auth: None,
            secure: false,
            ignore_tls: true,
            require_tls: false,
            name: Some("nudge.test".to_string()),
        }
    }

    #[test]
    fn dot_stuffing_protects_the_terminator() {
        assert_eq!(encode_data("hi\r\n.hidden\r\nbye"), "hi\r\n..hidden\r\nbye");
    }

    #[test]
    fn quoted_printable_encodes_non_ascii_and_equals() {
        assert_eq!(encode_quoted_printable("café"), "caf=C3=A9");
        assert_eq!(encode_quoted_printable("a=b"), "a=3Db");
        assert_eq!(encode_quoted_printable("one\ntwo"), "one=\r\ntwo");
    }

    #[test]
    fn quoted_printable_soft_wraps_long_lines() {
        let long = "a".repeat(100);
        let encoded = encode_quoted_printable(&long);

        for line in encoded.lines() {
            assert!(line.chars().count() <= 75);
        }
    }

    #[test]
    fn subject_rfc2047_when_not_ascii() {
        assert_eq!(encode_header_value("Plain"), "Plain");
        assert_eq!(
            encode_header_value("Ünicode"),
            format!("=?UTF-8?B?{}?=", BASE64.encode("Ünicode".as_bytes()))
        );
    }

    #[test]
    fn address_inner_extracts_envelope_address() {
        assert_eq!(
            address_inner("Felix from Twenty <noreply@yourdomain.com>"),
            "noreply@yourdomain.com"
        );
        assert_eq!(address_inner("a@b.com"), "a@b.com");
    }

    #[test]
    fn rfc822_carries_multipart_bodies() {
        let message = SendMailOptions {
            from: "Felix <felix@acme.example>".to_string(),
            to: vec!["user@target.example".to_string()],
            subject: "Test".to_string(),
            text: "plain body".to_string(),
            html: "<p>html body</p>".to_string(),
        };
        let raw = build_rfc822(&message, Utc::now());

        assert!(raw.starts_with("Date: "));
        assert!(raw.contains("From: Felix <felix@acme.example>"));
        assert!(raw.contains("Subject: Test"));
        assert!(raw.contains("Content-Type: multipart/alternative"));
        assert!(raw.contains("plain body"));
        assert!(raw.contains("<p>html body</p>"));
    }

    #[tokio::test]
    async fn delivers_a_full_session_over_a_local_listener() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = spawn_fake_smtp_server(
            listener,
            b"250-test greets\r\n250-SIZE 10485760\r\n250 AUTH PLAIN LOGIN\r\n",
        )
        .await;

        let message = SendMailOptions {
            from: "Felix from Twenty <noreply@example.test>".to_string(),
            to: vec!["recipient@target.test".to_string()],
            subject: "Welcome".to_string(),
            text: "Hello".to_string(),
            html: "<p>Hello</p>".to_string(),
        };

        send(&options(addr), &message).await.unwrap();

        let transcript = server.await.unwrap();
        assert!(transcript.contains("EHLO nudge.test\r\n"));
        assert!(transcript.contains("MAIL FROM:<noreply@example.test>\r\n"));
        assert!(transcript.contains("RCPT TO:<recipient@target.test>\r\n"));
        assert!(transcript.contains("Subject: Welcome"));
        assert!(transcript.contains("<p>Hello</p>"));
    }

    #[tokio::test]
    async fn authenticates_with_login_when_offered() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let expected_user_token = BASE64.encode("test-user");
        let expected_pass_token = BASE64.encode("test-pass-not-a-secret");

        let server =
            spawn_fake_smtp_server(listener, b"250-test\r\n250 AUTH LOGIN PLAIN\r\n").await;

        let transport = SmtpTransportOptions {
            auth: Some(crate::email::config::SmtpAuth {
                user: "test-user".to_string(),
                pass: "test-pass-not-a-secret".to_string(),
            }),
            ..options(addr)
        };

        send(
            &transport,
            &SendMailOptions {
                from: "a@example.test".to_string(),
                to: vec!["b@target.test".to_string()],
                subject: "s".to_string(),
                text: "t".to_string(),
                html: "h".to_string(),
            },
        )
        .await
        .unwrap();

        let transcript = server.await.unwrap();
        assert!(transcript.contains("AUTH LOGIN\r\n"));
        assert!(transcript.contains(&format!("{expected_user_token}\r\n")));
        assert!(transcript.contains(&format!("{expected_pass_token}\r\n")));
    }

    #[tokio::test]
    async fn authenticates_with_plain_when_login_is_not_offered() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let expected = BASE64.encode("\0test-user\0test-pass-not-a-secret");

        let server = spawn_fake_smtp_server(listener, b"250-test\r\n250 AUTH PLAIN\r\n").await;

        let transport = SmtpTransportOptions {
            auth: Some(crate::email::config::SmtpAuth {
                user: "test-user".to_string(),
                pass: "test-pass-not-a-secret".to_string(),
            }),
            ..options(addr)
        };

        send(
            &transport,
            &SendMailOptions {
                from: "a@example.test".to_string(),
                to: vec!["b@target.test".to_string()],
                subject: "s".to_string(),
                text: "t".to_string(),
                html: "h".to_string(),
            },
        )
        .await
        .unwrap();

        let transcript = server.await.unwrap();
        assert!(transcript.contains(&format!("AUTH PLAIN {expected}\r\n")));
    }

    #[tokio::test]
    async fn rejects_when_auth_required_but_not_advertised() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = spawn_fake_smtp_server(listener, b"250-test\r\n250 SIZE 10485760\r\n").await;

        let transport = SmtpTransportOptions {
            auth: Some(crate::email::config::SmtpAuth {
                user: "test-user".to_string(),
                pass: "test-pass-not-a-secret".to_string(),
            }),
            ..options(addr)
        };

        let result = send(
            &transport,
            &SendMailOptions {
                from: "a@example.test".to_string(),
                to: vec!["b@target.test".to_string()],
                subject: "s".to_string(),
                text: "t".to_string(),
                html: "h".to_string(),
            },
        )
        .await;

        assert_eq!(result, Err(SmtpError::AuthUnsupported));
        server.abort();
    }
}
