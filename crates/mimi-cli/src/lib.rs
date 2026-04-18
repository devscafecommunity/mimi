//! Beatrice CLI Library
//!
//! The command-line interface for MiMi, providing WebSocket and future HTTP/REPL interfaces.

pub mod auth;
pub mod ws;

pub use auth::*;
pub use ws::*;
