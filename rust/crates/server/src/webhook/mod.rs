#![allow(dead_code)]

pub mod dispatch;
pub mod matching;
pub mod payload;
pub mod rate_limit;
pub mod sender;
pub mod service;
pub mod signature;

pub use service::install as install_webhook_dispatcher;
