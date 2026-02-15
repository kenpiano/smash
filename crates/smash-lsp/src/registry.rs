//! LSP registry managing multiple language server clients.
//!
//! Maps language IDs to `LspClient` instances and orchestrates
//! server lifecycle across all active languages.
use std::collections::HashMap;

use crate::client::{ClientState, LspClient};
use crate::error::LspError;
use crate::types::{LspClientId, LspServerConfig};

/// Manages multiple LSP clients, one per language.
pub struct LspRegistry {
    /// Map of language ID to client.
    clients: HashMap<String, LspClient>,
    /// Counter for generating unique client IDs.
    next_id: u64,
}

impl LspRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            next_id: 1,
        }
    }

    /// Start a language server for the given configuration.
    ///
    /// Returns the client ID on success. Returns an error if a server
    /// for the same language is already running.
    pub async fn start_server(&mut self, config: LspServerConfig) -> Result<LspClientId, LspError> {
        let lang = config.language_id.clone();

        if let Some(existing) = self.clients.get(&lang) {
            if existing.state() == ClientState::Running {
                return Err(LspError::AlreadyRunning(lang));
            }
        }

        let id = LspClientId::new(self.next_id);
        self.next_id += 1;

        let mut client = LspClient::new(id, config);
        client.start().await?;
        self.clients.insert(lang, client);

        Ok(id)
    }

    /// Get a client by language ID.
    pub fn get(&self, language_id: &str) -> Option<&LspClient> {
        self.clients.get(language_id)
    }

    /// Get a mutable client by language ID.
    pub fn get_mut(&mut self, language_id: &str) -> Option<&mut LspClient> {
        self.clients.get_mut(language_id)
    }

    /// Check if a server is running for a given language.
    pub fn has_server(&self, language_id: &str) -> bool {
        self.clients
            .get(language_id)
            .is_some_and(|c| c.state() == ClientState::Running)
    }

    /// Get all active language IDs.
    pub fn active_languages(&self) -> Vec<&str> {
        self.clients
            .iter()
            .filter(|(_, c)| c.state() == ClientState::Running)
            .map(|(k, _)| k.as_str())
            .collect()
    }

    /// Number of registered clients (including stopped ones).
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }

    /// Shutdown a specific language server.
    pub async fn shutdown_server(&mut self, language_id: &str) -> Result<(), LspError> {
        if let Some(client) = self.clients.get_mut(language_id) {
            client.shutdown().await?;
            Ok(())
        } else {
            Err(LspError::NoServer(language_id.to_string()))
        }
    }

    /// Shutdown all language servers.
    pub async fn shutdown_all(&mut self) {
        let langs: Vec<String> = self.clients.keys().cloned().collect();
        for lang in langs {
            if let Some(client) = self.clients.get_mut(&lang) {
                let _ = client.shutdown().await;
            }
        }
    }
}

impl Default for LspRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for LspRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LspRegistry")
            .field("client_count", &self.clients.len())
            .field("next_id", &self.next_id)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_new_empty() {
        let reg = LspRegistry::new();
        assert_eq!(reg.client_count(), 0);
        assert!(reg.active_languages().is_empty());
    }

    #[test]
    fn registry_default_empty() {
        let reg = LspRegistry::default();
        assert_eq!(reg.client_count(), 0);
    }

    #[test]
    fn registry_get_nonexistent() {
        let reg = LspRegistry::new();
        assert!(reg.get("rust").is_none());
    }

    #[test]
    fn registry_has_server_false() {
        let reg = LspRegistry::new();
        assert!(!reg.has_server("rust"));
    }

    #[test]
    fn registry_debug_format() {
        let reg = LspRegistry::new();
        let debug = format!("{:?}", reg);
        assert!(debug.contains("LspRegistry"));
        assert!(debug.contains("client_count"));
    }

    #[tokio::test]
    async fn registry_shutdown_nonexistent() {
        let mut reg = LspRegistry::new();
        let result = reg.shutdown_server("rust").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            LspError::NoServer(lang) => assert_eq!(lang, "rust"),
            other => panic!("expected NoServer, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn registry_shutdown_all_empty() {
        let mut reg = LspRegistry::new();
        reg.shutdown_all().await; // Should not panic
    }

    #[tokio::test]
    async fn registry_start_invalid_command() {
        let mut reg = LspRegistry::new();
        let config = LspServerConfig {
            command: "nonexistent-lsp-server-xyz".to_string(),
            args: vec![],
            language_id: "test".to_string(),
            root_uri: None,
        };
        let result = reg.start_server(config).await;
        assert!(result.is_err());
    }
}
