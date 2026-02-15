use std::sync::Arc;

use tokio::sync::Mutex as TokioMutex;

use smash_lsp::LspRegistry;

use crate::lsp_types::{LspCommand, LspEvent};

/// Async task that manages LSP servers and processes commands.
///
/// Runs on the tokio runtime, receives commands from the main thread,
/// and sends events back via the event channel.
pub(crate) async fn lsp_manager_task(
    mut cmd_rx: tokio::sync::mpsc::Receiver<LspCommand>,
    evt_tx: std::sync::mpsc::Sender<LspEvent>,
) {
    let registry = Arc::new(TokioMutex::new(LspRegistry::new()));

    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            LspCommand::StartServer(config) => {
                handle_start_server(config, &registry, &evt_tx);
            }
            LspCommand::DidOpen {
                uri,
                text,
                language_id,
            } => {
                handle_did_open(uri, text, language_id, &registry, &evt_tx);
            }
            LspCommand::DidChange { uri, version, text } => {
                handle_did_change(uri, version, text, &registry);
            }
            LspCommand::DidSave { uri } => {
                handle_did_save(uri, &registry);
            }
            LspCommand::DidClose { uri } => {
                handle_did_close(uri, &registry);
            }
            LspCommand::Hover { uri, position } => {
                handle_hover(uri, position, &registry, &evt_tx);
            }
            LspCommand::GotoDefinition { uri, position } => {
                handle_goto_definition(uri, position, &registry, &evt_tx);
            }
            LspCommand::FindReferences { uri, position } => {
                handle_find_references(uri, position, &registry, &evt_tx);
            }
            LspCommand::Completion { uri, position } => {
                handle_completion(uri, position, &registry, &evt_tx);
            }
            LspCommand::Format { uri } => {
                handle_format(uri, &registry, &evt_tx);
            }
            LspCommand::CodeAction { uri, range } => {
                handle_code_action(uri, range, &registry, &evt_tx);
            }
            LspCommand::Shutdown => {
                let mut reg = registry.lock().await;
                reg.shutdown_all().await;
                break;
            }
        }
    }
}

// =========================================================================
// Individual command handlers (spawned as tokio tasks)
// =========================================================================

fn handle_start_server(
    config: smash_lsp::LspServerConfig,
    registry: &Arc<TokioMutex<LspRegistry>>,
    evt_tx: &std::sync::mpsc::Sender<LspEvent>,
) {
    let lang = config.language_id.clone();
    let registry = registry.clone();
    let evt_tx = evt_tx.clone();
    tokio::spawn(async move {
        let mut reg = registry.lock().await;
        match reg.start_server(config).await {
            Ok(_id) => {
                if let Some(client) = reg.get(&lang) {
                    let diag_store = client.diagnostics();
                    let diag_tx = evt_tx.clone();
                    diag_store.lock().await.set_on_update(move |uri, diags| {
                        let _ = diag_tx.send(LspEvent::DiagnosticsUpdated {
                            uri: uri.to_string(),
                            diagnostics: diags.to_vec(),
                        });
                    });
                }
                let _ = evt_tx.send(LspEvent::ServerStarted(lang));
            }
            Err(e) => {
                let _ = evt_tx.send(LspEvent::Error(format!(
                    "Failed to start LSP for {}: {}",
                    lang, e
                )));
            }
        }
    });
}

fn handle_did_open(
    uri: String,
    text: String,
    language_id: String,
    registry: &Arc<TokioMutex<LspRegistry>>,
    evt_tx: &std::sync::mpsc::Sender<LspEvent>,
) {
    let registry = registry.clone();
    let evt_tx = evt_tx.clone();
    let lang_id = language_id.clone();
    tokio::spawn(async move {
        let reg = registry.lock().await;
        if let Some(client) = reg.get(&lang_id) {
            if let Err(e) = client.did_open(&uri, &text, &lang_id).await {
                let _ = evt_tx.send(LspEvent::Error(format!("didOpen: {}", e)));
            }
        }
    });
}

fn handle_did_change(
    uri: String,
    version: i32,
    text: String,
    registry: &Arc<TokioMutex<LspRegistry>>,
) {
    let registry = registry.clone();
    tokio::spawn(async move {
        let reg = registry.lock().await;
        for lang in reg.active_languages() {
            if let Some(client) = reg.get(lang) {
                let _ = client.did_change(&uri, version, &text).await;
                break;
            }
        }
    });
}

fn handle_did_save(uri: String, registry: &Arc<TokioMutex<LspRegistry>>) {
    let registry = registry.clone();
    tokio::spawn(async move {
        let reg = registry.lock().await;
        for lang in reg.active_languages() {
            if let Some(client) = reg.get(lang) {
                let _ = client.did_save(&uri).await;
                break;
            }
        }
    });
}

fn handle_did_close(uri: String, registry: &Arc<TokioMutex<LspRegistry>>) {
    let registry = registry.clone();
    tokio::spawn(async move {
        let reg = registry.lock().await;
        for lang in reg.active_languages() {
            if let Some(client) = reg.get(lang) {
                let _ = client.did_close(&uri).await;
                break;
            }
        }
    });
}

fn handle_hover(
    uri: String,
    position: smash_lsp::LspPosition,
    registry: &Arc<TokioMutex<LspRegistry>>,
    evt_tx: &std::sync::mpsc::Sender<LspEvent>,
) {
    let registry = registry.clone();
    let evt_tx = evt_tx.clone();
    tokio::spawn(async move {
        let reg = registry.lock().await;
        for lang in reg.active_languages() {
            if let Some(client) = reg.get(lang) {
                match client.hover(&uri, position).await {
                    Ok(hover) => {
                        let text = hover.map(|h| h.contents.value);
                        let _ = evt_tx.send(LspEvent::HoverResult(text));
                    }
                    Err(e) => {
                        let _ = evt_tx.send(LspEvent::Error(format!("hover: {}", e)));
                    }
                }
                break;
            }
        }
    });
}

fn handle_goto_definition(
    uri: String,
    position: smash_lsp::LspPosition,
    registry: &Arc<TokioMutex<LspRegistry>>,
    evt_tx: &std::sync::mpsc::Sender<LspEvent>,
) {
    let registry = registry.clone();
    let evt_tx = evt_tx.clone();
    tokio::spawn(async move {
        let reg = registry.lock().await;
        for lang in reg.active_languages() {
            if let Some(client) = reg.get(lang) {
                match client.goto_definition(&uri, position).await {
                    Ok(locations) => {
                        let _ = evt_tx.send(LspEvent::GotoDefinitionResult(locations));
                    }
                    Err(e) => {
                        let _ = evt_tx.send(LspEvent::Error(format!("gotoDefinition: {}", e)));
                    }
                }
                break;
            }
        }
    });
}

fn handle_find_references(
    uri: String,
    position: smash_lsp::LspPosition,
    registry: &Arc<TokioMutex<LspRegistry>>,
    evt_tx: &std::sync::mpsc::Sender<LspEvent>,
) {
    let registry = registry.clone();
    let evt_tx = evt_tx.clone();
    tokio::spawn(async move {
        let reg = registry.lock().await;
        for lang in reg.active_languages() {
            if let Some(client) = reg.get(lang) {
                match client.find_references(&uri, position).await {
                    Ok(locations) => {
                        let _ = evt_tx.send(LspEvent::ReferencesResult(locations));
                    }
                    Err(e) => {
                        let _ = evt_tx.send(LspEvent::Error(format!("findReferences: {}", e)));
                    }
                }
                break;
            }
        }
    });
}

fn handle_completion(
    uri: String,
    position: smash_lsp::LspPosition,
    registry: &Arc<TokioMutex<LspRegistry>>,
    evt_tx: &std::sync::mpsc::Sender<LspEvent>,
) {
    let registry = registry.clone();
    let evt_tx = evt_tx.clone();
    tokio::spawn(async move {
        let reg = registry.lock().await;
        for lang in reg.active_languages() {
            if let Some(client) = reg.get(lang) {
                match client.completion(&uri, position).await {
                    Ok(items) => {
                        let _ = evt_tx.send(LspEvent::CompletionResult(items));
                    }
                    Err(e) => {
                        let _ = evt_tx.send(LspEvent::Error(format!("completion: {}", e)));
                    }
                }
                break;
            }
        }
    });
}

fn handle_format(
    uri: String,
    registry: &Arc<TokioMutex<LspRegistry>>,
    evt_tx: &std::sync::mpsc::Sender<LspEvent>,
) {
    let registry = registry.clone();
    let evt_tx = evt_tx.clone();
    tokio::spawn(async move {
        let reg = registry.lock().await;
        for lang in reg.active_languages() {
            if let Some(client) = reg.get(lang) {
                match client.format(&uri).await {
                    Ok(edits) => {
                        let _ = evt_tx.send(LspEvent::FormatResult(edits));
                    }
                    Err(e) => {
                        let _ = evt_tx.send(LspEvent::Error(format!("format: {}", e)));
                    }
                }
                break;
            }
        }
    });
}

fn handle_code_action(
    uri: String,
    range: smash_lsp::LspRange,
    registry: &Arc<TokioMutex<LspRegistry>>,
    evt_tx: &std::sync::mpsc::Sender<LspEvent>,
) {
    let registry = registry.clone();
    let evt_tx = evt_tx.clone();
    tokio::spawn(async move {
        let reg = registry.lock().await;
        for lang in reg.active_languages() {
            if let Some(client) = reg.get(lang) {
                match client.code_action(&uri, range, vec![]).await {
                    Ok(actions) => {
                        let _ = evt_tx.send(LspEvent::CodeActionResult(actions));
                    }
                    Err(e) => {
                        let _ = evt_tx.send(LspEvent::Error(format!("codeAction: {}", e)));
                    }
                }
                break;
            }
        }
    });
}
