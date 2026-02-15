use std::env;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{self, Event};
use tracing::{error, info};

use smash_config::load_config;
use smash_core::buffer::{Buffer, BufferId};
use smash_core::edit::EditCommand;
use smash_core::position::Position;
use smash_input::{create_default_keymap, Command, KeyResolver, Keymap, ResolveResult};
use smash_platform::paths::DefaultPaths;
use smash_platform::paths::PlatformPaths;
use smash_platform::Platform;
use smash_syntax::{LanguageId, RegexHighlighter};
use smash_tui::{default_dark_theme, PaneTree, Rect, Renderer, TerminalBackend, Viewport};

/// Application state
#[allow(dead_code)]
struct App {
    buffer: Buffer,
    viewport: Viewport,
    renderer: Renderer,
    panes: PaneTree,
    resolver: KeyResolver,
    highlighter: Option<RegexHighlighter>,
    filename: Option<String>,
    running: bool,
}

impl App {
    fn new(width: u16, height: u16, file: Option<PathBuf>) -> Result<Self> {
        let id = BufferId::next();
        let (buffer, filename, highlighter) = match file {
            Some(ref path) => {
                let buf = Buffer::open_or_create(id, path)
                    .with_context(|| format!("failed to open: {}", path.display()))?;
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unnamed")
                    .to_string();
                let lang = LanguageId::from_path(path);
                let hl = RegexHighlighter::new(lang).ok();
                (buf, Some(name), hl)
            }
            None => (Buffer::new(id), None, None),
        };

        // Reserve 1 line for status bar
        let edit_height = height.saturating_sub(1);

        let default_layer = create_default_keymap();
        let keymap = Keymap::new(default_layer);
        let resolver = KeyResolver::new(keymap);

        Ok(Self {
            buffer,
            viewport: Viewport::new(edit_height as usize, width as usize),
            renderer: Renderer::new(width, height),
            panes: PaneTree::new(),
            resolver,
            highlighter,
            filename,
            running: true,
        })
    }

    fn handle_command(&mut self, cmd: Command) {
        match cmd {
            Command::Quit | Command::ForceQuit => {
                self.running = false;
            }
            Command::InsertChar(c) => {
                let pos = self.buffer.cursors().primary().position();
                let edit = EditCommand::Insert {
                    pos,
                    text: c.to_string(),
                };
                if self.buffer.apply_edit(edit).is_ok() {
                    let new_pos = if c == '\n' {
                        Position::new(pos.line + 1, 0)
                    } else {
                        Position::new(pos.line, pos.col + 1)
                    };
                    self.buffer
                        .cursors_mut()
                        .primary_mut()
                        .set_position(new_pos);
                }
            }
            Command::InsertNewline => {
                self.handle_command(Command::InsertChar('\n'));
            }
            Command::DeleteBackward => {
                let pos = self.buffer.cursors().primary().position();
                if pos.col > 0 {
                    let start = Position::new(pos.line, pos.col - 1);
                    let range = smash_core::position::Range::new(start, pos);
                    let edit = EditCommand::Delete { range };
                    if self.buffer.apply_edit(edit).is_ok() {
                        self.buffer.cursors_mut().primary_mut().set_position(start);
                    }
                } else if pos.line > 0 {
                    let prev_line = pos.line - 1;
                    let prev_len = self
                        .buffer
                        .line(prev_line)
                        .map(|l| l.len_chars().saturating_sub(1))
                        .unwrap_or(0);
                    let start = Position::new(prev_line, prev_len);
                    let range = smash_core::position::Range::new(start, pos);
                    let edit = EditCommand::Delete { range };
                    if self.buffer.apply_edit(edit).is_ok() {
                        self.buffer.cursors_mut().primary_mut().set_position(start);
                    }
                }
            }
            Command::DeleteForward => {
                let pos = self.buffer.cursors().primary().position();
                let line_len = self
                    .buffer
                    .line(pos.line)
                    .map(|l| l.len_chars().saturating_sub(1))
                    .unwrap_or(0);
                let end = if pos.col < line_len {
                    Position::new(pos.line, pos.col + 1)
                } else if pos.line + 1 < self.buffer.line_count() {
                    Position::new(pos.line + 1, 0)
                } else {
                    return;
                };
                let range = smash_core::position::Range::new(pos, end);
                let edit = EditCommand::Delete { range };
                let _ = self.buffer.apply_edit(edit);
            }
            Command::MoveLeft => {
                let pos = self.buffer.cursors().primary().position();
                if pos.col > 0 {
                    self.buffer
                        .cursors_mut()
                        .primary_mut()
                        .set_position(Position::new(pos.line, pos.col - 1));
                }
            }
            Command::MoveRight => {
                let pos = self.buffer.cursors().primary().position();
                let line_len = self
                    .buffer
                    .line(pos.line)
                    .map(|l| l.len_chars().saturating_sub(1))
                    .unwrap_or(0);
                if pos.col < line_len {
                    self.buffer
                        .cursors_mut()
                        .primary_mut()
                        .set_position(Position::new(pos.line, pos.col + 1));
                }
            }
            Command::MoveUp => {
                let pos = self.buffer.cursors().primary().position();
                if pos.line > 0 {
                    self.buffer
                        .cursors_mut()
                        .primary_mut()
                        .set_position(Position::new(pos.line - 1, pos.col));
                }
            }
            Command::MoveDown => {
                let pos = self.buffer.cursors().primary().position();
                if pos.line + 1 < self.buffer.line_count() {
                    self.buffer
                        .cursors_mut()
                        .primary_mut()
                        .set_position(Position::new(pos.line + 1, pos.col));
                }
            }
            Command::MoveLineStart => {
                let pos = self.buffer.cursors().primary().position();
                self.buffer
                    .cursors_mut()
                    .primary_mut()
                    .set_position(Position::new(pos.line, 0));
            }
            Command::MoveLineEnd => {
                let pos = self.buffer.cursors().primary().position();
                let line_len = self
                    .buffer
                    .line(pos.line)
                    .map(|l| l.len_chars().saturating_sub(1))
                    .unwrap_or(0);
                self.buffer
                    .cursors_mut()
                    .primary_mut()
                    .set_position(Position::new(pos.line, line_len));
            }
            Command::MoveBufferStart => {
                self.buffer
                    .cursors_mut()
                    .primary_mut()
                    .set_position(Position::new(0, 0));
            }
            Command::MoveBufferEnd => {
                let last = self.buffer.line_count().saturating_sub(1);
                self.buffer
                    .cursors_mut()
                    .primary_mut()
                    .set_position(Position::new(last, 0));
            }
            Command::PageUp => {
                let lines = self.viewport.visible_lines();
                self.viewport.scroll_up(lines);
                let pos = self.buffer.cursors().primary().position();
                let new_line = pos.line.saturating_sub(lines);
                self.buffer
                    .cursors_mut()
                    .primary_mut()
                    .set_position(Position::new(new_line, pos.col));
            }
            Command::PageDown => {
                let lines = self.viewport.visible_lines();
                let total = self.buffer.line_count();
                self.viewport.scroll_down(lines, total);
                let pos = self.buffer.cursors().primary().position();
                let new_line = (pos.line + lines).min(total.saturating_sub(1));
                self.buffer
                    .cursors_mut()
                    .primary_mut()
                    .set_position(Position::new(new_line, pos.col));
            }
            Command::Undo => {
                let _ = self.buffer.undo();
            }
            Command::Redo => {
                let _ = self.buffer.redo();
            }
            Command::Save => {
                if self.buffer.path().is_some() {
                    match self.buffer.save() {
                        Ok(()) => {
                            info!("file saved");
                        }
                        Err(e) => {
                            error!("save failed: {}", e);
                        }
                    }
                }
            }
            _ => {
                // Commands not yet implemented in prototype
            }
        }
    }

    fn render(&mut self, backend: &mut dyn TerminalBackend) -> Result<()> {
        let (w, h) = backend.size()?;

        let pos = self.buffer.cursors().primary().position();
        self.viewport.scroll_to_cursor(pos.line, pos.col);

        let status_h = 1u16;
        let edit_area = Rect::new(0, 0, w, h.saturating_sub(status_h));
        let status_area = Rect::new(0, h.saturating_sub(status_h), w, status_h);

        let theme = default_dark_theme();

        self.renderer.render_buffer(
            &self.buffer,
            &self.viewport,
            edit_area,
            &theme,
            self.highlighter
                .as_ref()
                .map(|h| h as &dyn smash_syntax::HighlightEngine),
            true,
        );

        let fname = self.filename.as_deref().unwrap_or("[scratch]");
        self.renderer.render_status_bar(
            status_area,
            fname,
            pos.line,
            pos.col,
            self.buffer.is_dirty(),
            &theme,
        );

        self.renderer.flush_to_backend(backend)?;

        let gutter_w = 5u16;
        let screen_col = gutter_w + (pos.col.saturating_sub(self.viewport.left_col())) as u16;
        let screen_row = (pos.line.saturating_sub(self.viewport.top_line())) as u16;
        backend.move_cursor(screen_col, screen_row)?;
        backend.show_cursor()?;

        Ok(())
    }
}

fn run_editor(file: Option<PathBuf>) -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let paths = DefaultPaths::new().context("failed to detect platform paths")?;
    let config_dir = paths.config_dir();
    let project_dir = std::env::current_dir().ok();
    let _config = load_config(&config_dir, project_dir.as_deref()).unwrap_or_else(|e| {
        error!("config load failed, using defaults: {}", e);
        smash_config::Config::default()
    });

    let _platform = Platform::default_platform().context("failed to initialize platform")?;

    let (width, height) = crossterm::terminal::size().context("failed to get terminal size")?;

    let mut app = App::new(width, height, file)?;

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

    while app.running {
        if event::poll(Duration::from_millis(100))? {
            let raw_event = event::read()?;

            if let Event::Resize(w, h) = raw_event {
                app.viewport
                    .resize(h.saturating_sub(1) as usize, w as usize);
                app.renderer.resize(w, h);
                if let Err(e) = app.render(&mut backend) {
                    error!("render error: {}", e);
                }
                continue;
            }

            if let Some(input) = smash_input::event::from_crossterm(raw_event) {
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

            if let Err(e) = app.render(&mut backend) {
                error!("render error: {}", e);
            }
        }
    }

    crossterm::execute!(
        std::io::stdout(),
        crossterm::event::DisableMouseCapture,
        crossterm::terminal::LeaveAlternateScreen
    )?;
    crossterm::terminal::disable_raw_mode()?;

    info!("smash exited cleanly");
    Ok(())
}

/// Minimal crossterm backend for production use
struct CrosstermBackend {
    stdout: std::io::Stdout,
}

impl CrosstermBackend {
    fn new() -> Self {
        Self {
            stdout: std::io::stdout(),
        }
    }
}

impl TerminalBackend for CrosstermBackend {
    fn size(&self) -> Result<(u16, u16), smash_tui::TuiError> {
        crossterm::terminal::size().map_err(smash_tui::TuiError::Io)
    }

    fn move_cursor(&mut self, col: u16, row: u16) -> Result<(), smash_tui::TuiError> {
        use crossterm::cursor::MoveTo;
        crossterm::execute!(self.stdout, MoveTo(col, row)).map_err(smash_tui::TuiError::Io)
    }

    fn show_cursor(&mut self) -> Result<(), smash_tui::TuiError> {
        crossterm::execute!(self.stdout, crossterm::cursor::Show).map_err(smash_tui::TuiError::Io)
    }

    fn hide_cursor(&mut self) -> Result<(), smash_tui::TuiError> {
        crossterm::execute!(self.stdout, crossterm::cursor::Hide).map_err(smash_tui::TuiError::Io)
    }

    fn clear(&mut self) -> Result<(), smash_tui::TuiError> {
        use crossterm::terminal::{Clear, ClearType};
        crossterm::execute!(self.stdout, Clear(ClearType::All)).map_err(smash_tui::TuiError::Io)
    }

    fn write_cell(
        &mut self,
        col: u16,
        row: u16,
        cell: &smash_tui::Cell,
    ) -> Result<(), smash_tui::TuiError> {
        use crossterm::cursor::MoveTo;
        use crossterm::style::{Print, SetBackgroundColor, SetForegroundColor};
        let fg = to_crossterm_color(cell.style.fg);
        let bg = to_crossterm_color(cell.style.bg);
        crossterm::execute!(
            self.stdout,
            MoveTo(col, row),
            SetForegroundColor(fg),
            SetBackgroundColor(bg),
            Print(cell.ch)
        )
        .map_err(smash_tui::TuiError::Io)
    }

    fn flush(&mut self) -> Result<(), smash_tui::TuiError> {
        use std::io::Write;
        self.stdout.flush().map_err(smash_tui::TuiError::Io)
    }

    fn enter_alternate_screen(&mut self) -> Result<(), smash_tui::TuiError> {
        Ok(())
    }

    fn leave_alternate_screen(&mut self) -> Result<(), smash_tui::TuiError> {
        Ok(())
    }

    fn enable_raw_mode(&mut self) -> Result<(), smash_tui::TuiError> {
        Ok(())
    }

    fn disable_raw_mode(&mut self) -> Result<(), smash_tui::TuiError> {
        Ok(())
    }
}

fn to_crossterm_color(color: smash_tui::Color) -> crossterm::style::Color {
    match color {
        smash_tui::Color::Reset => crossterm::style::Color::Reset,
        smash_tui::Color::Black => crossterm::style::Color::Black,
        smash_tui::Color::Red => crossterm::style::Color::DarkRed,
        smash_tui::Color::Green => crossterm::style::Color::DarkGreen,
        smash_tui::Color::Yellow => crossterm::style::Color::DarkYellow,
        smash_tui::Color::Blue => crossterm::style::Color::DarkBlue,
        smash_tui::Color::Magenta => crossterm::style::Color::DarkMagenta,
        smash_tui::Color::Cyan => crossterm::style::Color::DarkCyan,
        smash_tui::Color::White => crossterm::style::Color::White,
        smash_tui::Color::Rgb(r, g, b) => crossterm::style::Color::Rgb { r, g, b },
        smash_tui::Color::Indexed(i) => crossterm::style::Color::AnsiValue(i),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let file = args.get(1).map(PathBuf::from);

    if let Err(e) = run_editor(file) {
        eprintln!("smash: {:#}", e);
        std::process::exit(1);
    }
}
