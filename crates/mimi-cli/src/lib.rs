//! Beatrice CLI Library
//! 
//! The command-line interface for MiMi, providing:
//! - Command parsing and validation
//! - Task execution (exec, query)
//! - Configuration management
//! - Debugging tools
//! - Interactive REPL mode

pub mod cli;
pub mod config;
pub mod logger;
pub mod parser;

pub use cli::commands;
pub use cli::handler;
pub use cli::formatter;
pub use cli::error;

pub use config::loader;
pub use config::defaults;

pub fn init_logging(verbose: u8, log_level: Option<&str>, no_color: bool) {
    logger::init_logging(verbose, log_level, no_color);
}
