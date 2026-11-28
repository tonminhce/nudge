pub mod config;
pub mod sender;
pub mod smtp;
pub mod templates;

pub use config::EmailDriver;
pub use config::EmailSendConfig;
pub use config::SmtpTransportOptions;
pub use sender::EMAIL_SEND_RETRY_LIMIT;
pub use smtp::SendMailOptions;
pub use templates::RenderedEmail;
