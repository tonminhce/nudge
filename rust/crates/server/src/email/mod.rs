#![allow(dead_code)]

pub mod config;
pub mod sender;
pub mod service;
pub mod smtp;
pub mod templates;

pub use service::init_from_env;
