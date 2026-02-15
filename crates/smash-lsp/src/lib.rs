//! smash-lsp — Language Server Protocol client for SMASH.
//!
//! This crate implements the LSP client for communicating with language
//! servers. It handles process management, JSON-RPC transport, and
//! request/response dispatch.
pub mod client;
pub mod diagnostics;
pub mod dispatcher;
pub mod error;
pub mod registry;
pub mod transport;
pub mod types;

// Re-export key types for convenience.
pub use client::{ClientState, LspClient};
pub use diagnostics::DiagnosticStore;
pub use error::LspError;
pub use registry::LspRegistry;
pub use types::{
    CodeAction, CompletionItem, CompletionItemKind, Diagnostic, DiagnosticSeverity, Hover,
    Location, LspCapabilities, LspClientId, LspPosition, LspRange, LspServerConfig, MarkupContent,
    SymbolInformation, SymbolKind, TextEdit, WorkspaceEdit,
};
