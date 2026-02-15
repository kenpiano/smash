//! LSP protocol types.
//!
//! These types mirror the Language Server Protocol specification (v3.17+)
//! and are used for communication between the editor and language servers.
use serde::{Deserialize, Serialize};

/// Opaque identifier for an LSP client instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LspClientId(u64);

impl LspClientId {
    /// Create a new client ID.
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Return the raw numeric id.
    pub fn raw(&self) -> u64 {
        self.0
    }
}

/// Configuration for launching an LSP server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspServerConfig {
    /// Executable command name.
    pub command: String,
    /// Command-line arguments.
    pub args: Vec<String>,
    /// Language identifier (e.g. "rust", "python").
    pub language_id: String,
    /// Root URI of the workspace.
    pub root_uri: Option<String>,
}

/// LSP Position — 0-based line and character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LspPosition {
    /// Zero-based line number.
    pub line: u32,
    /// Zero-based character offset (UTF-16).
    pub character: u32,
}

impl LspPosition {
    /// Create a new LSP position.
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

/// Convert from smash_core Position to LSP Position.
impl From<smash_core::position::Position> for LspPosition {
    fn from(pos: smash_core::position::Position) -> Self {
        Self {
            line: pos.line as u32,
            character: pos.col as u32,
        }
    }
}

/// Convert from LSP Position to smash_core Position.
impl From<LspPosition> for smash_core::position::Position {
    fn from(pos: LspPosition) -> Self {
        Self::new(pos.line as usize, pos.character as usize)
    }
}

/// LSP Range — start and end positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LspRange {
    /// Start position (inclusive).
    pub start: LspPosition,
    /// End position (exclusive).
    pub end: LspPosition,
}

impl LspRange {
    /// Create a new LSP range.
    pub fn new(start: LspPosition, end: LspPosition) -> Self {
        Self { start, end }
    }
}

/// Diagnostic severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    /// Reports an error.
    Error = 1,
    /// Reports a warning.
    Warning = 2,
    /// Reports an information.
    Information = 3,
    /// Reports a hint.
    Hint = 4,
}

/// A diagnostic message from the language server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// The range at which the diagnostic applies.
    pub range: LspRange,
    /// The severity of the diagnostic.
    pub severity: Option<DiagnosticSeverity>,
    /// The diagnostic's message.
    pub message: String,
    /// The diagnostic's source (e.g. "rustc", "clippy").
    pub source: Option<String>,
    /// The diagnostic's code (string or number).
    pub code: Option<String>,
}

/// Completion item kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompletionItemKind {
    /// A text completion.
    Text = 1,
    /// A method completion.
    Method = 2,
    /// A function completion.
    Function = 3,
    /// A constructor completion.
    Constructor = 4,
    /// A field completion.
    Field = 5,
    /// A variable completion.
    Variable = 6,
    /// A class completion.
    Class = 7,
    /// An interface completion.
    Interface = 8,
    /// A module completion.
    Module = 9,
    /// A property completion.
    Property = 10,
    /// A keyword completion.
    Keyword = 14,
    /// A snippet completion.
    Snippet = 15,
}

/// A completion item returned by the language server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionItem {
    /// The label of this completion item.
    pub label: String,
    /// The kind of this completion item.
    pub kind: Option<CompletionItemKind>,
    /// A human-readable string with additional information.
    pub detail: Option<String>,
    /// A string that should be inserted into a document when selecting
    /// this completion.
    pub insert_text: Option<String>,
    /// The documentation for this completion item.
    pub documentation: Option<String>,
}

/// Markup content for hover results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkupContent {
    /// The type of the markup content.
    pub kind: String,
    /// The content itself.
    pub value: String,
}

/// Hover information from the language server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hover {
    /// The hover's content.
    pub contents: MarkupContent,
    /// An optional range.
    pub range: Option<LspRange>,
}

/// A location in a document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    /// The URI of the document.
    pub uri: String,
    /// The range within the document.
    pub range: LspRange,
}

/// Symbol kinds used in document symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKind {
    /// A file symbol.
    File = 1,
    /// A module symbol.
    Module = 2,
    /// A namespace symbol.
    Namespace = 3,
    /// A class symbol.
    Class = 5,
    /// A method symbol.
    Method = 6,
    /// A property symbol.
    Property = 7,
    /// A field symbol.
    Field = 8,
    /// A constructor symbol.
    Constructor = 9,
    /// An enum symbol.
    Enum = 10,
    /// A function symbol.
    Function = 12,
    /// A variable symbol.
    Variable = 13,
    /// A constant symbol.
    Constant = 14,
    /// A struct symbol.
    Struct = 23,
}

/// Information about a symbol in a document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolInformation {
    /// The name of this symbol.
    pub name: String,
    /// The kind of this symbol.
    pub kind: SymbolKind,
    /// The location of this symbol.
    pub location: Location,
    /// The name of the containing symbol.
    pub container_name: Option<String>,
}

/// A text edit to be applied to a document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEdit {
    /// The range of the text document to be manipulated.
    pub range: LspRange,
    /// The string to be inserted.
    pub new_text: String,
}

/// A code action returned by the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeAction {
    /// A short, human-readable title.
    pub title: String,
    /// The kind of the code action.
    pub kind: Option<String>,
    /// The workspace edit this code action performs.
    pub edit: Option<WorkspaceEdit>,
}

/// A workspace edit represents changes to many resources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorkspaceEdit {
    /// Holds changes to existing resources, keyed by URI.
    pub changes: Option<std::collections::HashMap<String, Vec<TextEdit>>>,
}

/// Negotiated capabilities after initialization.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LspCapabilities {
    /// Server supports completion.
    pub completion: bool,
    /// Server supports hover.
    pub hover: bool,
    /// Server supports go-to-definition.
    pub goto_definition: bool,
    /// Server supports find-references.
    pub find_references: bool,
    /// Server supports code actions.
    pub code_actions: bool,
    /// Server supports document formatting.
    pub formatting: bool,
    /// Server supports rename.
    pub rename: bool,
    /// Server supports document symbols.
    pub document_symbols: bool,
    /// Server supports diagnostics (always assumed if server runs).
    pub diagnostics: bool,
    /// Server supports signature help.
    pub signature_help: bool,
}

impl LspCapabilities {
    /// Create capabilities from server initialization result.
    pub fn from_server_capabilities(caps: &serde_json::Value) -> Self {
        Self {
            completion: caps.get("completionProvider").is_some(),
            hover: caps
                .get("hoverProvider")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
                || caps.get("hoverProvider").is_some_and(|v| v.is_object()),
            goto_definition: caps
                .get("definitionProvider")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
                || caps
                    .get("definitionProvider")
                    .is_some_and(|v| v.is_object()),
            find_references: caps
                .get("referencesProvider")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
                || caps
                    .get("referencesProvider")
                    .is_some_and(|v| v.is_object()),
            code_actions: caps
                .get("codeActionProvider")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
                || caps
                    .get("codeActionProvider")
                    .is_some_and(|v| v.is_object()),
            formatting: caps
                .get("documentFormattingProvider")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
                || caps
                    .get("documentFormattingProvider")
                    .is_some_and(|v| v.is_object()),
            rename: caps.get("renameProvider").is_some(),
            document_symbols: caps
                .get("documentSymbolProvider")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
                || caps
                    .get("documentSymbolProvider")
                    .is_some_and(|v| v.is_object()),
            diagnostics: true, // Always assumed
            signature_help: caps.get("signatureHelpProvider").is_some(),
        }
    }
}

/// Client capabilities sent during initialization.
pub fn client_capabilities() -> serde_json::Value {
    serde_json::json!({
        "textDocument": {
            "completion": {
                "completionItem": {
                    "snippetSupport": false,
                    "documentationFormat": ["plaintext"]
                }
            },
            "hover": {
                "contentFormat": ["plaintext"]
            },
            "definition": {
                "dynamicRegistration": false,
                "linkSupport": false
            },
            "references": {
                "dynamicRegistration": false
            },
            "codeAction": {
                "dynamicRegistration": false
            },
            "formatting": {
                "dynamicRegistration": false
            },
            "rename": {
                "dynamicRegistration": false,
                "prepareSupport": false
            },
            "publishDiagnostics": {
                "relatedInformation": false
            },
            "documentSymbol": {
                "dynamicRegistration": false,
                "hierarchicalDocumentSymbolSupport": false
            },
            "signatureHelp": {
                "dynamicRegistration": false
            },
            "synchronization": {
                "didSave": true,
                "willSave": false,
                "dynamicRegistration": false
            }
        },
        "workspace": {
            "workspaceFolders": false,
            "configuration": false
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lsp_client_id_new_and_raw() {
        let id = LspClientId::new(42);
        assert_eq!(id.raw(), 42);
    }

    #[test]
    fn lsp_client_id_equality() {
        let a = LspClientId::new(1);
        let b = LspClientId::new(1);
        let c = LspClientId::new(2);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn lsp_client_id_clone_copy() {
        let a = LspClientId::new(5);
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn lsp_position_new() {
        let pos = LspPosition::new(10, 20);
        assert_eq!(pos.line, 10);
        assert_eq!(pos.character, 20);
    }

    #[test]
    fn lsp_position_default() {
        let pos = LspPosition::default();
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn lsp_position_from_core() {
        let core_pos = smash_core::position::Position::new(5, 10);
        let lsp_pos: LspPosition = core_pos.into();
        assert_eq!(lsp_pos.line, 5);
        assert_eq!(lsp_pos.character, 10);
    }

    #[test]
    fn core_position_from_lsp() {
        let lsp_pos = LspPosition::new(3, 7);
        let core_pos: smash_core::position::Position = lsp_pos.into();
        assert_eq!(core_pos.line, 3);
        assert_eq!(core_pos.col, 7);
    }

    #[test]
    fn lsp_range_new() {
        let range = LspRange::new(LspPosition::new(1, 0), LspPosition::new(1, 10));
        assert_eq!(range.start.line, 1);
        assert_eq!(range.end.character, 10);
    }

    #[test]
    fn lsp_range_default() {
        let range = LspRange::default();
        assert_eq!(range.start, LspPosition::default());
        assert_eq!(range.end, LspPosition::default());
    }

    #[test]
    fn diagnostic_severity_values() {
        assert_eq!(DiagnosticSeverity::Error as i32, 1);
        assert_eq!(DiagnosticSeverity::Warning as i32, 2);
        assert_eq!(DiagnosticSeverity::Information as i32, 3);
        assert_eq!(DiagnosticSeverity::Hint as i32, 4);
    }

    #[test]
    fn diagnostic_serialize_deserialize() {
        let diag = Diagnostic {
            range: LspRange::new(LspPosition::new(0, 0), LspPosition::new(0, 5)),
            severity: Some(DiagnosticSeverity::Error),
            message: "undefined variable".to_string(),
            source: Some("rustc".to_string()),
            code: Some("E0425".to_string()),
        };
        let json = serde_json::to_string(&diag).unwrap();
        let deser: Diagnostic = serde_json::from_str(&json).unwrap();
        assert_eq!(deser, diag);
    }

    #[test]
    fn diagnostic_without_optional_fields() {
        let diag = Diagnostic {
            range: LspRange::default(),
            severity: None,
            message: "something wrong".into(),
            source: None,
            code: None,
        };
        let json = serde_json::to_string(&diag).unwrap();
        let deser: Diagnostic = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.severity, None);
        assert_eq!(deser.source, None);
    }

    #[test]
    fn completion_item_serialize_deserialize() {
        let item = CompletionItem {
            label: "println!".to_string(),
            kind: Some(CompletionItemKind::Function),
            detail: Some("macro".to_string()),
            insert_text: Some("println!(\"$1\")".to_string()),
            documentation: Some("println docs".to_string()),
        };
        let json = serde_json::to_string(&item).unwrap();
        let deser: CompletionItem = serde_json::from_str(&json).unwrap();
        assert_eq!(deser, item);
    }

    #[test]
    fn completion_item_kind_values() {
        assert_eq!(CompletionItemKind::Text as i32, 1);
        assert_eq!(CompletionItemKind::Method as i32, 2);
        assert_eq!(CompletionItemKind::Function as i32, 3);
        assert_eq!(CompletionItemKind::Variable as i32, 6);
        assert_eq!(CompletionItemKind::Keyword as i32, 14);
    }

    #[test]
    fn hover_serialize_deserialize() {
        let hover = Hover {
            contents: MarkupContent {
                kind: "plaintext".to_string(),
                value: "fn main()".to_string(),
            },
            range: Some(LspRange::new(
                LspPosition::new(0, 0),
                LspPosition::new(0, 4),
            )),
        };
        let json = serde_json::to_string(&hover).unwrap();
        let deser: Hover = serde_json::from_str(&json).unwrap();
        assert_eq!(deser, hover);
    }

    #[test]
    fn location_serialize_deserialize() {
        let loc = Location {
            uri: "file:///src/main.rs".to_string(),
            range: LspRange::new(LspPosition::new(10, 0), LspPosition::new(10, 20)),
        };
        let json = serde_json::to_string(&loc).unwrap();
        let deser: Location = serde_json::from_str(&json).unwrap();
        assert_eq!(deser, loc);
    }

    #[test]
    fn symbol_kind_values() {
        assert_eq!(SymbolKind::File as i32, 1);
        assert_eq!(SymbolKind::Function as i32, 12);
        assert_eq!(SymbolKind::Struct as i32, 23);
    }

    #[test]
    fn symbol_information_roundtrip() {
        let sym = SymbolInformation {
            name: "main".to_string(),
            kind: SymbolKind::Function,
            location: Location {
                uri: "file:///src/main.rs".to_string(),
                range: LspRange::new(LspPosition::new(5, 0), LspPosition::new(10, 1)),
            },
            container_name: None,
        };
        let json = serde_json::to_string(&sym).unwrap();
        let deser: SymbolInformation = serde_json::from_str(&json).unwrap();
        assert_eq!(deser, sym);
    }

    #[test]
    fn text_edit_roundtrip() {
        let edit = TextEdit {
            range: LspRange::new(LspPosition::new(0, 0), LspPosition::new(0, 5)),
            new_text: "hello".to_string(),
        };
        let json = serde_json::to_string(&edit).unwrap();
        let deser: TextEdit = serde_json::from_str(&json).unwrap();
        assert_eq!(deser, edit);
    }

    #[test]
    fn code_action_roundtrip() {
        let action = CodeAction {
            title: "Remove unused import".to_string(),
            kind: Some("quickfix".to_string()),
            edit: None,
        };
        let json = serde_json::to_string(&action).unwrap();
        let deser: CodeAction = serde_json::from_str(&json).unwrap();
        assert_eq!(deser, action);
    }

    #[test]
    fn workspace_edit_default() {
        let edit = WorkspaceEdit::default();
        assert!(edit.changes.is_none());
    }

    #[test]
    fn workspace_edit_with_changes() {
        let mut changes = std::collections::HashMap::new();
        changes.insert(
            "file:///src/main.rs".to_string(),
            vec![TextEdit {
                range: LspRange::default(),
                new_text: "new".to_string(),
            }],
        );
        let edit = WorkspaceEdit {
            changes: Some(changes),
        };
        let json = serde_json::to_string(&edit).unwrap();
        let deser: WorkspaceEdit = serde_json::from_str(&json).unwrap();
        assert_eq!(deser, edit);
    }

    #[test]
    fn capabilities_from_full_server() {
        let caps = serde_json::json!({
            "completionProvider": { "triggerCharacters": ["."] },
            "hoverProvider": true,
            "definitionProvider": true,
            "referencesProvider": true,
            "codeActionProvider": true,
            "documentFormattingProvider": true,
            "renameProvider": { "prepareProvider": true },
            "documentSymbolProvider": true,
            "signatureHelpProvider": { "triggerCharacters": ["(", ","] }
        });
        let lsp_caps = LspCapabilities::from_server_capabilities(&caps);
        assert!(lsp_caps.completion);
        assert!(lsp_caps.hover);
        assert!(lsp_caps.goto_definition);
        assert!(lsp_caps.find_references);
        assert!(lsp_caps.code_actions);
        assert!(lsp_caps.formatting);
        assert!(lsp_caps.rename);
        assert!(lsp_caps.document_symbols);
        assert!(lsp_caps.diagnostics);
        assert!(lsp_caps.signature_help);
    }

    #[test]
    fn capabilities_from_minimal_server() {
        let caps = serde_json::json!({
            "hoverProvider": true
        });
        let lsp_caps = LspCapabilities::from_server_capabilities(&caps);
        assert!(!lsp_caps.completion);
        assert!(lsp_caps.hover);
        assert!(!lsp_caps.goto_definition);
        assert!(!lsp_caps.find_references);
        assert!(!lsp_caps.code_actions);
        assert!(!lsp_caps.formatting);
        assert!(!lsp_caps.rename);
        assert!(!lsp_caps.document_symbols);
        assert!(lsp_caps.diagnostics); // Always true
        assert!(!lsp_caps.signature_help);
    }

    #[test]
    fn capabilities_from_empty_server() {
        let caps = serde_json::json!({});
        let lsp_caps = LspCapabilities::from_server_capabilities(&caps);
        assert!(!lsp_caps.completion);
        assert!(!lsp_caps.hover);
        assert!(lsp_caps.diagnostics);
    }

    #[test]
    fn capabilities_object_providers() {
        let caps = serde_json::json!({
            "hoverProvider": {},
            "definitionProvider": {},
            "referencesProvider": {},
            "codeActionProvider": {},
            "documentFormattingProvider": {},
            "documentSymbolProvider": {}
        });
        let lsp_caps = LspCapabilities::from_server_capabilities(&caps);
        assert!(lsp_caps.hover);
        assert!(lsp_caps.goto_definition);
        assert!(lsp_caps.find_references);
        assert!(lsp_caps.code_actions);
        assert!(lsp_caps.formatting);
        assert!(lsp_caps.document_symbols);
    }

    #[test]
    fn capabilities_default() {
        let caps = LspCapabilities::default();
        assert!(!caps.completion);
        assert!(!caps.hover);
        assert!(!caps.goto_definition);
        assert!(!caps.diagnostics);
    }

    #[test]
    fn client_capabilities_has_completion() {
        let caps = client_capabilities();
        assert!(caps["textDocument"]["completion"].is_object());
    }

    #[test]
    fn client_capabilities_has_hover() {
        let caps = client_capabilities();
        assert!(caps["textDocument"]["hover"].is_object());
    }

    #[test]
    fn client_capabilities_has_definition() {
        let caps = client_capabilities();
        assert!(caps["textDocument"]["definition"].is_object());
    }

    #[test]
    fn client_capabilities_has_publish_diagnostics() {
        let caps = client_capabilities();
        assert!(caps["textDocument"]["publishDiagnostics"].is_object());
    }

    #[test]
    fn client_capabilities_has_synchronization() {
        let caps = client_capabilities();
        let sync = &caps["textDocument"]["synchronization"];
        assert_eq!(sync["didSave"], true);
    }

    #[test]
    fn server_config_fields() {
        let config = LspServerConfig {
            command: "rust-analyzer".to_string(),
            args: vec!["--stdio".to_string()],
            language_id: "rust".to_string(),
            root_uri: Some("file:///project".to_string()),
        };
        assert_eq!(config.command, "rust-analyzer");
        assert_eq!(config.args.len(), 1);
        assert_eq!(config.language_id, "rust");
        assert!(config.root_uri.is_some());
    }

    #[test]
    fn server_config_clone() {
        let config = LspServerConfig {
            command: "pyright".to_string(),
            args: vec![],
            language_id: "python".to_string(),
            root_uri: None,
        };
        let cloned = config.clone();
        assert_eq!(cloned, config);
    }

    #[test]
    fn lsp_position_serialize() {
        let pos = LspPosition::new(5, 10);
        let json = serde_json::to_string(&pos).unwrap();
        assert!(json.contains("\"line\":5"));
        assert!(json.contains("\"character\":10"));
    }

    #[test]
    fn markup_content_roundtrip() {
        let mc = MarkupContent {
            kind: "markdown".to_string(),
            value: "# Hello".to_string(),
        };
        let json = serde_json::to_string(&mc).unwrap();
        let deser: MarkupContent = serde_json::from_str(&json).unwrap();
        assert_eq!(deser, mc);
    }
}
