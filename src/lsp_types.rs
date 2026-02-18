use smash_lsp::{CompletionItem, Diagnostic, LspPosition, LspRange, LspServerConfig};

/// Events sent from the async LSP task back to the main thread.
#[allow(dead_code)]
pub(crate) enum LspEvent {
    /// LSP server started for a language.
    ServerStarted(String),
    /// Hover result (text to display).
    HoverResult(Option<String>),
    /// Go-to-definition result (locations).
    GotoDefinitionResult(Vec<smash_lsp::Location>),
    /// Find-references result.
    ReferencesResult(Vec<smash_lsp::Location>),
    /// Completion result.
    CompletionResult(Vec<CompletionItem>),
    /// Format result (text edits).
    FormatResult(Vec<smash_lsp::TextEdit>),
    /// Code actions available.
    CodeActionResult(Vec<smash_lsp::CodeAction>),
    /// Diagnostics updated for a URI.
    DiagnosticsUpdated {
        uri: String,
        diagnostics: Vec<Diagnostic>,
    },
    /// Error message from LSP.
    Error(String),
    /// Info message from LSP.
    Info(String),
}

/// Commands sent from the main thread to the async LSP task.
#[allow(dead_code)]
pub(crate) enum LspCommand {
    StartServer(LspServerConfig),
    DidOpen {
        uri: String,
        text: String,
        language_id: String,
    },
    DidChange {
        uri: String,
        version: i32,
        text: String,
    },
    DidSave {
        uri: String,
    },
    DidClose {
        uri: String,
    },
    Hover {
        uri: String,
        position: LspPosition,
    },
    GotoDefinition {
        uri: String,
        position: LspPosition,
    },
    FindReferences {
        uri: String,
        position: LspPosition,
    },
    Completion {
        uri: String,
        position: LspPosition,
    },
    Format {
        uri: String,
    },
    CodeAction {
        uri: String,
        range: LspRange,
    },
    Shutdown,
}
