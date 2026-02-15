//! smash-dap — Debug Adapter Protocol client for SMASH.
//!
//! This crate implements the DAP client for communicating with debug
//! adapters. It handles protocol types, message framing, session
//! lifecycle, and breakpoint management.

pub mod breakpoint;
pub mod capabilities;
pub mod client;
pub mod error;
pub mod protocol;
pub mod session;
pub mod transport;

// Re-export key types for convenience.
pub use breakpoint::{Breakpoint, BreakpointManager};
pub use capabilities::DapCapabilities;
pub use client::DapClient;
pub use error::DapError;
pub use protocol::*;
pub use session::{DapSession, SessionState};
