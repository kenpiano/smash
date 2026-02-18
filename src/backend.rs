use smash_tui::TerminalBackend;

/// Minimal crossterm backend for production use.
pub(crate) struct CrosstermBackend {
    stdout: std::io::Stdout,
}

impl CrosstermBackend {
    pub(crate) fn new() -> Self {
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
