use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{self, Event};
use tracing::{error, info};

use smash_config::load_config;
use smash_input::ResolveResult;
use smash_platform::paths::DefaultPaths;
use smash_platform::paths::PlatformPaths;
use smash_platform::Platform;

use crate::app::{App, InputMode};
use crate::backend::CrosstermBackend;
use crate::lsp_types::LspCommand;

/// Set up the editor, run the event loop, and clean up on exit.
pub(crate) fn run_editor(file: Option<PathBuf>) -> Result<()> {
    let paths = DefaultPaths::new().context("failed to detect platform paths")?;

    // Load configuration first so we can honour log settings.
    let config_dir = paths.config_dir();
    let project_dir = std::env::current_dir().ok();
    let config = load_config(&config_dir, project_dir.as_deref())
        .unwrap_or_else(|_e| smash_config::Config::default());

    // ── Logging initialisation (REQ-NFR-020, REQ-NFR-021) ──────────────────
    let log_path = config.log.file.clone().unwrap_or_else(|| {
        let dir = paths.log_dir();
        dir.join("smash.log")
    });

    smash_core::logging::ensure_log_dir(&log_path).ok();
    smash_core::logging::rotate_log_files(
        &log_path,
        smash_core::logging::DEFAULT_MAX_LOG_SIZE,
        smash_core::logging::DEFAULT_MAX_LOG_FILES,
    )
    .ok();

    let filter_str = match &config.log.level {
        smash_config::config::LogLevel::Trace => "trace",
        smash_config::config::LogLevel::Debug => "debug",
        smash_config::config::LogLevel::Info => "info",
        smash_config::config::LogLevel::Warn => "warn",
        smash_config::config::LogLevel::Error => "error",
    };

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .unwrap_or_else(|_| std::fs::File::create("/dev/null").expect("cannot open /dev/null"));

    let env_filter = tracing_subscriber::EnvFilter::try_new(filter_str)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_writer(std::sync::Mutex::new(log_file))
        .with_ansi(false)
        .with_env_filter(env_filter)
        .init();

    info!("smash starting – log level: {}", filter_str);

    let _platform = Platform::default_platform().context("failed to initialize platform")?;

    let (width, height) = crossterm::terminal::size().context("failed to get terminal size")?;

    // Set up LSP channels
    let (lsp_cmd_tx, lsp_cmd_rx) = tokio::sync::mpsc::channel::<LspCommand>(64);
    let (lsp_evt_tx, lsp_evt_rx) = std::sync::mpsc::channel();

    // Start tokio runtime for async LSP operations
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .context("failed to create tokio runtime")?;

    runtime.spawn(crate::lsp_task::lsp_manager_task(lsp_cmd_rx, lsp_evt_tx));

    let mut app = App::new(
        width,
        height,
        file,
        &config.keymap.preset,
        lsp_cmd_tx.clone(),
        lsp_evt_rx,
        config.lsp.enabled,
        config.lsp.servers.clone(),
        config.editor.option_as_alt,
    )?;

    // Start LSP for initial file if configured
    app.start_lsp_for_current_file();

    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;

    let mut backend = CrosstermBackend::new();

    if let Err(e) = app.render(&mut backend) {
        error!("render error: {}", e);
    }

    run_event_loop(&mut app, &mut backend)?;

    crossterm::execute!(
        std::io::stdout(),
        crossterm::event::DisableMouseCapture,
        crossterm::terminal::LeaveAlternateScreen
    )?;
    crossterm::terminal::disable_raw_mode()?;

    // Shutdown LSP servers
    let _ = lsp_cmd_tx.try_send(LspCommand::Shutdown);
    drop(lsp_cmd_tx);
    runtime.shutdown_timeout(Duration::from_secs(2));

    info!("smash exited cleanly");
    Ok(())
}

/// Main event loop — poll for terminal events and LSP updates.
fn run_event_loop(app: &mut App, backend: &mut CrosstermBackend) -> Result<()> {
    while app.running {
        // Drain any pending LSP events
        let mut had_lsp_event = false;
        while let Ok(evt) = app.lsp_evt_rx.try_recv() {
            app.handle_lsp_event(evt);
            had_lsp_event = true;
        }
        if had_lsp_event {
            if let Err(e) = app.render(backend) {
                error!("render error: {}", e);
            }
        }

        if event::poll(Duration::from_millis(50))? {
            let raw_event = event::read()?;

            if let Event::Resize(w, h) = raw_event {
                app.viewport
                    .resize(h.saturating_sub(1) as usize, w as usize);
                app.renderer.resize(w, h);
                if let Err(e) = app.render(backend) {
                    error!("render error: {}", e);
                }
                continue;
            }

            if let Some(input) = smash_input::event::from_crossterm(raw_event) {
                let input = if app.option_as_alt {
                    match input {
                        smash_input::InputEvent::Key(ke) => smash_input::InputEvent::Key(
                            smash_input::event::normalize_macos_option_key(ke),
                        ),
                        other => other,
                    }
                } else {
                    input
                };

                // Handle Esc to cancel prompts
                if let smash_input::InputEvent::Key(ke) = &input {
                    if ke.key == smash_input::Key::Esc && app.input_mode != InputMode::Normal {
                        app.input_mode = InputMode::Normal;
                        app.prompt_input.clear();
                        if let Err(e) = app.render(backend) {
                            error!("render error: {}", e);
                        }
                        continue;
                    }
                }

                match app.resolver.resolve(input) {
                    ResolveResult::Command(cmd) => {
                        app.handle_command(cmd);
                    }
                    ResolveResult::WaitingForMore => {
                        continue;
                    }
                    ResolveResult::Unresolved => {}
                }
            }

            if let Err(e) = app.render(backend) {
                error!("render error: {}", e);
            }
        }
    }
    Ok(())
}
