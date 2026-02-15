//! LSP client managing a single language server process.
//!
//! Handles lifecycle (spawn, initialize, shutdown), request/response
//! interchange, and notification routing.
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command as TokioCommand};
use tokio::sync::{mpsc, Mutex};
use tokio::time::{timeout, Duration};

use crate::diagnostics::DiagnosticStore;
use crate::dispatcher::{DispatchResult, Dispatcher};
use crate::error::LspError;
use crate::transport::{
    frame_message, next_request_id, parse_message, serialize_notification, serialize_request,
    JsonRpcMessage,
};
use crate::types::{
    client_capabilities, CodeAction, CompletionItem, Diagnostic, Hover, Location, LspCapabilities,
    LspClientId, LspPosition, LspRange, LspServerConfig, SymbolInformation, TextEdit,
    WorkspaceEdit,
};

/// Default timeout for requests (seconds).
const REQUEST_TIMEOUT_SECS: u64 = 10;

/// State of the LSP client lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientState {
    /// Client created but not yet initialized.
    Created,
    /// Initialize handshake in progress.
    Initializing,
    /// Ready to handle requests.
    Running,
    /// Shutting down.
    ShuttingDown,
    /// Stopped.
    Stopped,
}

/// An LSP client connected to a single language server.
pub struct LspClient {
    id: LspClientId,
    config: LspServerConfig,
    state: ClientState,
    capabilities: LspCapabilities,
    dispatcher: Arc<Mutex<Dispatcher>>,
    writer_tx: Option<mpsc::Sender<Vec<u8>>>,
    child: Option<Child>,
    diagnostics: Arc<Mutex<DiagnosticStore>>,
}

impl LspClient {
    /// Create a new LSP client with the given configuration.
    pub fn new(id: LspClientId, config: LspServerConfig) -> Self {
        Self {
            id,
            config,
            state: ClientState::Created,
            capabilities: LspCapabilities::default(),
            dispatcher: Arc::new(Mutex::new(Dispatcher::new())),
            writer_tx: None,
            child: None,
            diagnostics: Arc::new(Mutex::new(DiagnosticStore::new())),
        }
    }

    /// Get the client ID.
    pub fn id(&self) -> LspClientId {
        self.id
    }

    /// Get the current state.
    pub fn state(&self) -> ClientState {
        self.state
    }

    /// Get the negotiated capabilities.
    pub fn capabilities(&self) -> &LspCapabilities {
        &self.capabilities
    }

    /// Get the server config.
    pub fn config(&self) -> &LspServerConfig {
        &self.config
    }

    /// Get access to the diagnostic store.
    pub fn diagnostics(&self) -> Arc<Mutex<DiagnosticStore>> {
        self.diagnostics.clone()
    }

    /// Start the language server process and perform initialization.
    pub async fn start(&mut self) -> Result<(), LspError> {
        self.state = ClientState::Initializing;

        // Spawn the server process
        let mut child = TokioCommand::new(&self.config.command)
            .args(&self.config.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| LspError::SpawnFailed(format!("{}: {}", self.config.command, e)))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| LspError::SpawnFailed("could not capture stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| LspError::SpawnFailed("could not capture stdout".into()))?;

        // Writer task: sends messages to the server
        let (writer_tx, mut writer_rx) = mpsc::channel::<Vec<u8>>(64);
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(msg) = writer_rx.recv().await {
                if stdin.write_all(&msg).await.is_err() {
                    break;
                }
                if stdin.flush().await.is_err() {
                    break;
                }
            }
        });

        // Reader task: reads messages from the server
        let dispatcher = self.dispatcher.clone();
        let diagnostics = self.diagnostics.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut buf = String::new();

            loop {
                buf.clear();
                // Read header lines until empty line
                let mut content_length: Option<usize> = None;
                loop {
                    let mut line = String::new();
                    match reader.read_line(&mut line).await {
                        Ok(0) => return, // EOF
                        Err(_) => return,
                        Ok(_) => {}
                    }
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        break;
                    }
                    if let Some(val) = trimmed.strip_prefix("Content-Length:") {
                        content_length = val.trim().parse().ok();
                    }
                }

                let length = match content_length {
                    Some(l) => l,
                    None => continue,
                };

                // Read body
                let mut body_buf = vec![0u8; length];
                match tokio::io::AsyncReadExt::read_exact(&mut reader, &mut body_buf).await {
                    Ok(_) => {}
                    Err(_) => return,
                }

                let body = match String::from_utf8(body_buf) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                let message = match parse_message(&body) {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                // Handle diagnostics notifications specially
                if let JsonRpcMessage::Notification {
                    ref method,
                    ref params,
                } = message
                {
                    if method == "textDocument/publishDiagnostics" {
                        if let (Some(uri), Some(diag_arr)) =
                            (params["uri"].as_str(), params["diagnostics"].as_array())
                        {
                            let diags: Vec<Diagnostic> = diag_arr
                                .iter()
                                .filter_map(|d| serde_json::from_value(d.clone()).ok())
                                .collect();
                            diagnostics.lock().await.publish(uri.to_string(), diags);
                        }
                    }
                }

                let mut disp = dispatcher.lock().await;
                let _ = disp.dispatch(message);
            }
        });

        self.writer_tx = Some(writer_tx);
        self.child = Some(child);

        // Perform initialize handshake
        self.initialize().await?;

        self.state = ClientState::Running;
        Ok(())
    }

    /// Send the initialize request to the server.
    async fn initialize(&mut self) -> Result<(), LspError> {
        let params = serde_json::json!({
            "processId": std::process::id(),
            "capabilities": client_capabilities(),
            "rootUri": self.config.root_uri,
            "clientInfo": {
                "name": "smash",
                "version": "0.1.0"
            }
        });

        let result = self.send_request("initialize", params).await?;

        // Parse server capabilities
        if let Some(caps) = result.get("capabilities") {
            self.capabilities = LspCapabilities::from_server_capabilities(caps);
        }

        // Send initialized notification
        self.send_notification("initialized", serde_json::json!({}))
            .await?;

        Ok(())
    }

    /// Send a request and wait for the response.
    pub async fn send_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, LspError> {
        let writer_tx = self.writer_tx.as_ref().ok_or(LspError::ServerCrashed)?;

        let id = next_request_id();
        let body = serialize_request(id, method, params);
        let framed = frame_message(&body);

        let rx = {
            let mut disp = self.dispatcher.lock().await;
            disp.register_request(id)
        };

        writer_tx
            .send(framed)
            .await
            .map_err(|_| LspError::ServerCrashed)?;

        let result = timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS), rx)
            .await
            .map_err(|_| LspError::Timeout(REQUEST_TIMEOUT_SECS))?
            .map_err(|_| LspError::ServerCrashed)?;

        match result {
            DispatchResult::Success(val) => Ok(val),
            DispatchResult::Error(err) => Err(LspError::Rpc {
                code: err.code,
                message: err.message,
            }),
        }
    }

    /// Send a notification (no response expected).
    pub async fn send_notification(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<(), LspError> {
        let writer_tx = self.writer_tx.as_ref().ok_or(LspError::ServerCrashed)?;

        let body = serialize_notification(method, params);
        let framed = frame_message(&body);

        writer_tx
            .send(framed)
            .await
            .map_err(|_| LspError::ServerCrashed)?;

        Ok(())
    }

    /// Send textDocument/didOpen notification.
    pub async fn did_open(&self, uri: &str, text: &str, language_id: &str) -> Result<(), LspError> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri,
                "languageId": language_id,
                "version": 1,
                "text": text
            }
        });
        self.send_notification("textDocument/didOpen", params).await
    }

    /// Send textDocument/didChange notification.
    pub async fn did_change(&self, uri: &str, version: i32, text: &str) -> Result<(), LspError> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri,
                "version": version
            },
            "contentChanges": [{
                "text": text
            }]
        });
        self.send_notification("textDocument/didChange", params)
            .await
    }

    /// Send textDocument/didSave notification.
    pub async fn did_save(&self, uri: &str) -> Result<(), LspError> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri
            }
        });
        self.send_notification("textDocument/didSave", params).await
    }

    /// Send textDocument/didClose notification.
    pub async fn did_close(&self, uri: &str) -> Result<(), LspError> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri
            }
        });
        self.send_notification("textDocument/didClose", params)
            .await
    }

    /// Request completions at a position.
    pub async fn completion(
        &self,
        uri: &str,
        position: LspPosition,
    ) -> Result<Vec<CompletionItem>, LspError> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": position
        });
        let result = self.send_request("textDocument/completion", params).await?;

        // Response may be CompletionList or CompletionItem[]
        let items_value = if result.is_array() {
            result
        } else if let Some(items) = result.get("items") {
            items.clone()
        } else {
            return Ok(Vec::new());
        };

        let items: Vec<CompletionItem> = serde_json::from_value(items_value)
            .map_err(|e| LspError::Serialization(format!("completion parse: {}", e)))?;
        Ok(items)
    }

    /// Request hover information at a position.
    pub async fn hover(&self, uri: &str, position: LspPosition) -> Result<Option<Hover>, LspError> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": position
        });
        let result = self.send_request("textDocument/hover", params).await?;

        if result.is_null() {
            return Ok(None);
        }

        let hover: Hover = serde_json::from_value(result)
            .map_err(|e| LspError::Serialization(format!("hover parse: {}", e)))?;
        Ok(Some(hover))
    }

    /// Request go-to-definition.
    pub async fn goto_definition(
        &self,
        uri: &str,
        position: LspPosition,
    ) -> Result<Vec<Location>, LspError> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": position
        });
        let result = self.send_request("textDocument/definition", params).await?;

        if result.is_null() {
            return Ok(Vec::new());
        }

        // May be Location, Location[], or LocationLink[]
        if result.is_array() {
            let locations: Vec<Location> = serde_json::from_value(result)
                .map_err(|e| LspError::Serialization(format!("definition parse: {}", e)))?;
            Ok(locations)
        } else {
            let location: Location = serde_json::from_value(result)
                .map_err(|e| LspError::Serialization(format!("definition parse: {}", e)))?;
            Ok(vec![location])
        }
    }

    /// Request find-references.
    pub async fn find_references(
        &self,
        uri: &str,
        position: LspPosition,
    ) -> Result<Vec<Location>, LspError> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": position,
            "context": { "includeDeclaration": true }
        });
        let result = self.send_request("textDocument/references", params).await?;

        if result.is_null() {
            return Ok(Vec::new());
        }

        let locations: Vec<Location> = serde_json::from_value(result)
            .map_err(|e| LspError::Serialization(format!("references parse: {}", e)))?;
        Ok(locations)
    }

    /// Request code actions for a range.
    pub async fn code_action(
        &self,
        uri: &str,
        range: LspRange,
        diagnostics: Vec<Diagnostic>,
    ) -> Result<Vec<CodeAction>, LspError> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "range": range,
            "context": { "diagnostics": diagnostics }
        });
        let result = self.send_request("textDocument/codeAction", params).await?;

        if result.is_null() {
            return Ok(Vec::new());
        }

        let actions: Vec<CodeAction> = serde_json::from_value(result)
            .map_err(|e| LspError::Serialization(format!("code action parse: {}", e)))?;
        Ok(actions)
    }

    /// Request document formatting.
    pub async fn format(&self, uri: &str) -> Result<Vec<TextEdit>, LspError> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "options": {
                "tabSize": 4,
                "insertSpaces": true
            }
        });
        let result = self.send_request("textDocument/formatting", params).await?;

        if result.is_null() {
            return Ok(Vec::new());
        }

        let edits: Vec<TextEdit> = serde_json::from_value(result)
            .map_err(|e| LspError::Serialization(format!("format parse: {}", e)))?;
        Ok(edits)
    }

    /// Request rename.
    pub async fn rename(
        &self,
        uri: &str,
        position: LspPosition,
        new_name: &str,
    ) -> Result<WorkspaceEdit, LspError> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": position,
            "newName": new_name
        });
        let result = self.send_request("textDocument/rename", params).await?;

        let edit: WorkspaceEdit = serde_json::from_value(result)
            .map_err(|e| LspError::Serialization(format!("rename parse: {}", e)))?;
        Ok(edit)
    }

    /// Request document symbols.
    pub async fn document_symbols(&self, uri: &str) -> Result<Vec<SymbolInformation>, LspError> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri }
        });
        let result = self
            .send_request("textDocument/documentSymbol", params)
            .await?;

        if result.is_null() {
            return Ok(Vec::new());
        }

        let symbols: Vec<SymbolInformation> = serde_json::from_value(result)
            .map_err(|e| LspError::Serialization(format!("symbol parse: {}", e)))?;
        Ok(symbols)
    }

    /// Shutdown the language server.
    pub async fn shutdown(&mut self) -> Result<(), LspError> {
        if self.state == ClientState::Stopped {
            return Ok(());
        }

        self.state = ClientState::ShuttingDown;

        // Send shutdown request
        let _result = self.send_request("shutdown", serde_json::Value::Null).await;

        // Send exit notification
        let _ = self
            .send_notification("exit", serde_json::Value::Null)
            .await;

        // Drop the writer channel
        self.writer_tx = None;

        // Wait for the child
        if let Some(ref mut child) = self.child {
            let _ = child.wait().await;
        }

        self.state = ClientState::Stopped;
        Ok(())
    }
}

impl std::fmt::Debug for LspClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LspClient")
            .field("id", &self.id)
            .field("config", &self.config)
            .field("state", &self.state)
            .field("capabilities", &self.capabilities)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> LspServerConfig {
        LspServerConfig {
            command: "echo".to_string(),
            args: vec![],
            language_id: "test".to_string(),
            root_uri: Some("file:///test".to_string()),
        }
    }

    #[test]
    fn client_new_state() {
        let client = LspClient::new(LspClientId::new(1), test_config());
        assert_eq!(client.state(), ClientState::Created);
        assert_eq!(client.id(), LspClientId::new(1));
    }

    #[test]
    fn client_config_access() {
        let config = test_config();
        let client = LspClient::new(LspClientId::new(1), config.clone());
        assert_eq!(client.config().command, "echo");
        assert_eq!(client.config().language_id, "test");
    }

    #[test]
    fn client_capabilities_initially_empty() {
        let client = LspClient::new(LspClientId::new(1), test_config());
        assert!(!client.capabilities().completion);
        assert!(!client.capabilities().hover);
    }

    #[test]
    fn client_debug_format() {
        let client = LspClient::new(LspClientId::new(1), test_config());
        let debug = format!("{:?}", client);
        assert!(debug.contains("LspClient"));
        assert!(debug.contains("Created"));
    }

    #[tokio::test]
    async fn client_diagnostics_store_accessible() {
        let client = LspClient::new(LspClientId::new(1), test_config());
        let store = client.diagnostics();
        let locked = store.lock().await;
        assert_eq!(locked.total_count(), 0);
    }

    #[test]
    fn client_state_variants() {
        assert_eq!(ClientState::Created, ClientState::Created);
        assert_ne!(ClientState::Created, ClientState::Running);
        assert_ne!(ClientState::Running, ClientState::Stopped);
        assert_ne!(ClientState::Initializing, ClientState::ShuttingDown);
    }

    #[test]
    fn client_state_debug() {
        let state = ClientState::Running;
        let debug = format!("{:?}", state);
        assert_eq!(debug, "Running");
    }

    #[test]
    fn client_state_clone() {
        let state = ClientState::Running;
        let cloned = state;
        assert_eq!(state, cloned);
    }

    #[tokio::test]
    async fn client_spawn_nonexistent_command() {
        let config = LspServerConfig {
            command: "definitely-not-a-real-command-xyz".to_string(),
            args: vec![],
            language_id: "test".to_string(),
            root_uri: None,
        };
        let mut client = LspClient::new(LspClientId::new(1), config);
        let result = client.start().await;
        assert!(result.is_err());
        match result.unwrap_err() {
            LspError::SpawnFailed(msg) => {
                assert!(msg.contains("definitely-not-a-real-command-xyz"));
            }
            other => panic!("expected SpawnFailed, got: {:?}", other),
        }
    }
}
