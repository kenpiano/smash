use std::path::PathBuf;

use thiserror::Error;

use crate::position::{ByteOffset, Position};

/// Errors that can occur during editing operations.
#[derive(Debug, Error)]
pub enum EditError {
    #[error(
        "position ({line}, {col}) is out of buffer bounds",
        line = .0.line,
        col = .0.col
    )]
    OutOfBounds(Position),

    #[error("invalid byte range {start}..{end}", start = .start.0, end = .end.0)]
    InvalidRange { start: ByteOffset, end: ByteOffset },

    #[error("encoding error: {0}")]
    Encoding(String),

    #[error("file I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("swap file corrupted")]
    SwapFileCorrupted,

    #[error("file not found: {0}")]
    FileNotFound(PathBuf),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_error_out_of_bounds_displays_correctly() {
        let err = EditError::OutOfBounds(Position::new(5, 10));
        assert_eq!(err.to_string(), "position (5, 10) is out of buffer bounds");
    }

    #[test]
    fn edit_error_invalid_range_displays_correctly() {
        let err = EditError::InvalidRange {
            start: ByteOffset(10),
            end: ByteOffset(5),
        };
        assert_eq!(err.to_string(), "invalid byte range 10..5");
    }

    #[test]
    fn edit_error_encoding_displays_correctly() {
        let err = EditError::Encoding("invalid UTF-8".to_string());
        assert_eq!(err.to_string(), "encoding error: invalid UTF-8");
    }

    #[test]
    fn edit_error_io_from_std_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let err: EditError = io_err.into();
        assert!(err.to_string().contains("file I/O error"));
    }

    #[test]
    fn edit_error_swap_file_corrupted_displays() {
        let err = EditError::SwapFileCorrupted;
        assert_eq!(err.to_string(), "swap file corrupted");
    }

    #[test]
    fn edit_error_file_not_found_displays_path() {
        let err = EditError::FileNotFound(PathBuf::from("/tmp/missing.txt"));
        assert_eq!(err.to_string(), "file not found: /tmp/missing.txt");
    }
}
