use crate::error::{TerminalError, TerminalResult};
use crate::grid::TerminalSize;

/// Trait abstracting a PTY (pseudo-terminal) for testability.
pub trait Pty: std::fmt::Debug + Send {
    /// Write data to the PTY input (stdin of the child process).
    fn write(&mut self, data: &[u8]) -> TerminalResult<()>;

    /// Read available data from the PTY output (stdout of the child process).
    /// Returns an empty vec if no data is available.
    fn read(&mut self) -> TerminalResult<Vec<u8>>;

    /// Resize the PTY to the given dimensions.
    fn resize(&mut self, size: TerminalSize) -> TerminalResult<()>;

    /// Check if the child process is still running.
    fn is_alive(&self) -> bool;

    /// Get the exit code if the process has exited.
    fn exit_code(&self) -> Option<i32>;

    /// Terminate the PTY and child process.
    fn close(&mut self) -> TerminalResult<()>;
}

/// A mock PTY for testing purposes.
/// Stores written data and allows setting read data.
#[derive(Debug)]
pub struct MockPty {
    /// Data that has been written to the PTY.
    pub written: Vec<u8>,
    /// Data to be returned on the next read.
    pub read_buffer: Vec<u8>,
    /// Current size.
    pub size: TerminalSize,
    /// Whether the mock process is alive.
    pub alive: bool,
    /// Exit code (set when not alive).
    pub exit: Option<i32>,
}

impl MockPty {
    /// Create a new mock PTY with the given size.
    pub fn new(size: TerminalSize) -> Self {
        Self {
            written: Vec::new(),
            read_buffer: Vec::new(),
            size,
            alive: true,
            exit: None,
        }
    }

    /// Set the data that will be returned on the next read.
    pub fn set_read_data(&mut self, data: &[u8]) {
        self.read_buffer = data.to_vec();
    }

    /// Simulate process exit with the given code.
    pub fn simulate_exit(&mut self, code: i32) {
        self.alive = false;
        self.exit = Some(code);
    }
}

impl Pty for MockPty {
    fn write(&mut self, data: &[u8]) -> TerminalResult<()> {
        if !self.alive {
            return Err(TerminalError::Io("PTY is closed".to_string()));
        }
        self.written.extend_from_slice(data);
        Ok(())
    }

    fn read(&mut self) -> TerminalResult<Vec<u8>> {
        if !self.alive && self.read_buffer.is_empty() {
            return Err(TerminalError::Io("PTY is closed".to_string()));
        }
        let data = std::mem::take(&mut self.read_buffer);
        Ok(data)
    }

    fn resize(&mut self, size: TerminalSize) -> TerminalResult<()> {
        self.size = size;
        Ok(())
    }

    fn is_alive(&self) -> bool {
        self.alive
    }

    fn exit_code(&self) -> Option<i32> {
        self.exit
    }

    fn close(&mut self) -> TerminalResult<()> {
        self.alive = false;
        self.exit = Some(0);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_pty_write_and_read() {
        let size = TerminalSize::new(80, 24);
        let mut pty = MockPty::new(size);

        // Write data
        pty.write(b"hello").unwrap();
        assert_eq!(pty.written, b"hello");

        // Set read data and read it back
        pty.set_read_data(b"world");
        let data = pty.read().unwrap();
        assert_eq!(data, b"world");

        // Second read returns empty (data was consumed)
        let data = pty.read().unwrap();
        assert!(data.is_empty());
    }

    #[test]
    fn mock_pty_resize() {
        let size = TerminalSize::new(80, 24);
        let mut pty = MockPty::new(size);

        let new_size = TerminalSize::new(100, 50);
        pty.resize(new_size).unwrap();
        assert_eq!(pty.size, new_size);
    }

    #[test]
    fn mock_pty_lifecycle() {
        let size = TerminalSize::new(80, 24);
        let mut pty = MockPty::new(size);

        assert!(pty.is_alive());
        assert!(pty.exit_code().is_none());

        pty.close().unwrap();

        assert!(!pty.is_alive());
        assert_eq!(pty.exit_code(), Some(0));
    }

    #[test]
    fn mock_pty_write_after_close_fails() {
        let size = TerminalSize::new(80, 24);
        let mut pty = MockPty::new(size);
        pty.close().unwrap();

        let result = pty.write(b"test");
        assert!(result.is_err());
    }

    #[test]
    fn mock_pty_simulate_exit() {
        let size = TerminalSize::new(80, 24);
        let mut pty = MockPty::new(size);
        pty.simulate_exit(42);

        assert!(!pty.is_alive());
        assert_eq!(pty.exit_code(), Some(42));
    }
}
