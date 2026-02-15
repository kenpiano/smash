use crate::error::TerminalResult;
use crate::grid::{TerminalGrid, TerminalSize};
use crate::hyperlink::{DetectedLink, HyperlinkDetector};
use crate::input::key_to_escape_sequence;
use crate::parser::{TerminalEvent, VtParser};
use crate::pty::Pty;
use smash_input::KeyEvent;

/// A terminal pane that combines a PTY, VT parser, grid, and hyperlink detector.
#[derive(Debug)]
pub struct TerminalPane {
    /// The underlying PTY.
    pty: Box<dyn Pty>,
    /// The terminal character grid.
    grid: TerminalGrid,
    /// The VT escape sequence parser.
    parser: VtParser,
    /// Hyperlink detector.
    link_detector: HyperlinkDetector,
}

impl TerminalPane {
    /// Create a new terminal pane with the given PTY and size.
    pub fn new(pty: Box<dyn Pty>, size: TerminalSize) -> Self {
        Self {
            pty,
            grid: TerminalGrid::new(size.cols, size.rows),
            parser: VtParser::new(),
            link_detector: HyperlinkDetector::new(),
        }
    }

    /// Write raw input bytes to the PTY.
    pub fn write_input(&mut self, data: &[u8]) -> TerminalResult<()> {
        self.pty.write(data)
    }

    /// Translate a key event and send it to the PTY.
    pub fn send_key(&mut self, event: &KeyEvent) -> TerminalResult<()> {
        let seq = key_to_escape_sequence(event);
        if !seq.is_empty() {
            self.pty.write(&seq)?;
        }
        Ok(())
    }

    /// Read output from the PTY, parse it, and update the grid.
    /// Returns any terminal events generated during parsing.
    pub fn process_output(&mut self) -> TerminalResult<Vec<TerminalEvent>> {
        let data = self.pty.read()?;
        if data.is_empty() {
            return Ok(Vec::new());
        }
        let events = self.parser.process(&data, &mut self.grid);
        Ok(events)
    }

    /// Get a reference to the terminal grid.
    pub fn grid(&self) -> &TerminalGrid {
        &self.grid
    }

    /// Get a mutable reference to the terminal grid.
    pub fn grid_mut(&mut self) -> &mut TerminalGrid {
        &mut self.grid
    }

    /// Resize the terminal.
    pub fn resize(&mut self, size: TerminalSize) -> TerminalResult<()> {
        self.pty.resize(size)?;
        self.grid.resize(size.cols, size.rows);
        Ok(())
    }

    /// Detect hyperlinks in the current grid.
    pub fn detect_links(&self) -> Vec<DetectedLink> {
        self.link_detector.detect_in_grid(&self.grid)
    }

    /// Check if the terminal process is still alive.
    pub fn is_alive(&self) -> bool {
        self.pty.is_alive()
    }

    /// Get the exit code of the terminal process, if it has exited.
    pub fn exit_code(&self) -> Option<i32> {
        self.pty.exit_code()
    }

    /// Close the terminal (terminate the PTY process).
    pub fn close(&mut self) -> TerminalResult<()> {
        self.pty.close()
    }

    /// Get the current title, if set by an OSC sequence.
    pub fn title(&self) -> Option<&str> {
        self.grid.title.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pty::MockPty;
    use smash_input::{Key, Modifiers};

    fn create_test_pane(cols: u16, rows: u16) -> TerminalPane {
        let size = TerminalSize::new(cols, rows);
        let pty = Box::new(MockPty::new(size));
        TerminalPane::new(pty, size)
    }

    #[test]
    fn pane_creation() {
        let pane = create_test_pane(80, 24);
        assert_eq!(pane.grid().size(), TerminalSize::new(80, 24));
        assert!(pane.is_alive());
    }

    #[test]
    fn pane_write_input() {
        let mut pane = create_test_pane(80, 24);
        pane.write_input(b"hello").unwrap();
    }

    #[test]
    fn pane_send_key() {
        let mut pane = create_test_pane(80, 24);
        let event = KeyEvent::new(Key::Char('a'), Modifiers::NONE);
        pane.send_key(&event).unwrap();
    }

    #[test]
    fn pane_process_output() {
        let size = TerminalSize::new(80, 24);
        let mut mock = MockPty::new(size);
        mock.set_read_data(b"Hello");
        let mut pane = TerminalPane::new(Box::new(mock), size);

        let events = pane.process_output().unwrap();
        // Output was processed (no special events for plain text)
        assert!(events.is_empty());

        // Grid should show "Hello"
        assert_eq!(pane.grid().get_cell(0, 0).unwrap().character, 'H');
        assert_eq!(pane.grid().get_cell(0, 4).unwrap().character, 'o');
    }

    #[test]
    fn pane_process_output_with_title() {
        let size = TerminalSize::new(80, 24);
        let mut mock = MockPty::new(size);
        mock.set_read_data(b"\x1b]0;Test Title\x07");
        let mut pane = TerminalPane::new(Box::new(mock), size);

        let events = pane.process_output().unwrap();
        assert!(events.contains(&TerminalEvent::TitleChanged("Test Title".to_string())));
        assert_eq!(pane.title(), Some("Test Title"));
    }

    #[test]
    fn pane_resize() {
        let mut pane = create_test_pane(80, 24);
        let new_size = TerminalSize::new(120, 40);
        pane.resize(new_size).unwrap();
        assert_eq!(pane.grid().size(), new_size);
    }

    #[test]
    fn pane_close() {
        let mut pane = create_test_pane(80, 24);
        assert!(pane.is_alive());
        pane.close().unwrap();
        assert!(!pane.is_alive());
        assert_eq!(pane.exit_code(), Some(0));
    }

    #[test]
    fn pane_detect_links() {
        let size = TerminalSize::new(80, 24);
        let mut mock = MockPty::new(size);
        mock.set_read_data(b"Visit https://example.com today");
        let mut pane = TerminalPane::new(Box::new(mock), size);
        pane.process_output().unwrap();

        let links = pane.detect_links();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].uri, "https://example.com");
    }

    #[test]
    fn pane_empty_read() {
        let mut pane = create_test_pane(80, 24);
        // No data set — read returns empty
        let events = pane.process_output().unwrap();
        assert!(events.is_empty());
    }
}
