//! Beatrice CLI Library
//!
//! The command-line interface for MiMi, providing:
//! - AI adapter management and performance monitoring
//! - Authentication and security
//! - WebSocket communication
//! - Command parsing and validation
//! - Task execution (exec, query)
//! - Configuration management
//! - Debugging tools
//! - Interactive REPL mode

pub mod ai;
pub mod auth;
pub mod cli;
pub mod config;
pub mod logger;
pub mod parser;
pub mod ws;

pub use ai::{AdapterConfig, AdapterError, AdapterResult, AiAdapter};
pub use auth::*;
pub use cli::commands;
pub use cli::error;
pub use cli::formatter;
pub use cli::handler;
pub use config::defaults;
pub use config::loader;
pub use ws::*;

pub fn init_logging(verbose: u8, log_level: Option<&str>, no_color: bool) {
    logger::init_logging(verbose, log_level, no_color);
}
