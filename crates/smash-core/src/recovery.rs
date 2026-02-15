use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::edit::{EditCommand, IndentDirection};
use crate::error::EditError;
use crate::position::{Position, Range};

/// Metadata and edit log stored in a swap file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapFileData {
    /// Hash of the original file content at time of last save.
    pub original_hash: String,
    /// The path to the original file.
    pub original_path: PathBuf,
    /// Log of edit commands since last save.
    pub edits: Vec<SerializableEditCommand>,
}

/// A serializable version of `EditCommand`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SerializableEditCommand {
    Insert {
        line: usize,
        col: usize,
        text: String,
    },
    Delete {
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
    },
    Replace {
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
        text: String,
    },
    Batch(Vec<SerializableEditCommand>),
}

/// Derive the swap file path from the original file path.
///
/// Example: `"/home/user/file.rs"` → `"/home/user/.file.rs.smash-swap"`.
pub fn swap_file_path(original: &Path) -> PathBuf {
    let file_name = original.file_name().unwrap_or_default().to_string_lossy();
    let swap_name = format!(".{file_name}.smash-swap");
    match original.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(swap_name),
        _ => PathBuf::from(swap_name),
    }
}

/// Compute a hash of `content` using `DefaultHasher` and return it as a hex string.
pub fn hash_content(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Write swap file data to disk atomically (write temp then rename).
pub fn write_swap_file(path: &Path, data: &SwapFileData) -> Result<(), EditError> {
    let json = serde_json::to_string(data).map_err(|e| {
        warn!("failed to serialize swap data: {e}");
        EditError::SwapFileCorrupted
    })?;
    let tmp_path = path.with_extension("smash-swap.tmp");
    std::fs::write(&tmp_path, json.as_bytes())?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

/// Read swap file data from disk.
pub fn read_swap_file(path: &Path) -> Result<SwapFileData, EditError> {
    let bytes = std::fs::read(path)?;
    let data: SwapFileData = serde_json::from_slice(&bytes).map_err(|e| {
        warn!("failed to deserialize swap file: {e}");
        EditError::SwapFileCorrupted
    })?;
    Ok(data)
}

/// Delete a swap file if it exists.
pub fn delete_swap_file(path: &Path) -> Result<(), EditError> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

/// Check if a swap file exists for the given original file.
pub fn has_swap_file(original: &Path) -> bool {
    swap_file_path(original).exists()
}

/// Convert an `EditCommand` into its serializable form.
pub fn to_serializable(cmd: &EditCommand) -> SerializableEditCommand {
    match cmd {
        EditCommand::Insert { pos, text } => SerializableEditCommand::Insert {
            line: pos.line,
            col: pos.col,
            text: text.clone(),
        },
        EditCommand::Delete { range } => SerializableEditCommand::Delete {
            start_line: range.start.line,
            start_col: range.start.col,
            end_line: range.end.line,
            end_col: range.end.col,
        },
        EditCommand::Replace { range, text } => SerializableEditCommand::Replace {
            start_line: range.start.line,
            start_col: range.start.col,
            end_line: range.end.line,
            end_col: range.end.col,
            text: text.clone(),
        },
        EditCommand::IndentLines { lines, direction } => {
            // Represent indent as a batch of inserts/deletes per line.
            let cmds: Vec<SerializableEditCommand> = lines
                .iter()
                .map(|&line| match direction {
                    IndentDirection::In => SerializableEditCommand::Insert {
                        line,
                        col: 0,
                        text: "    ".to_string(),
                    },
                    IndentDirection::Out => SerializableEditCommand::Delete {
                        start_line: line,
                        start_col: 0,
                        end_line: line,
                        end_col: 4,
                    },
                })
                .collect();
            SerializableEditCommand::Batch(cmds)
        }
        EditCommand::Batch(cmds) => {
            SerializableEditCommand::Batch(cmds.iter().map(to_serializable).collect())
        }
    }
}

/// Convert a `SerializableEditCommand` back into an `EditCommand`.
pub fn from_serializable(cmd: &SerializableEditCommand) -> EditCommand {
    match cmd {
        SerializableEditCommand::Insert { line, col, text } => EditCommand::Insert {
            pos: Position::new(*line, *col),
            text: text.clone(),
        },
        SerializableEditCommand::Delete {
            start_line,
            start_col,
            end_line,
            end_col,
        } => EditCommand::Delete {
            range: Range::new(
                Position::new(*start_line, *start_col),
                Position::new(*end_line, *end_col),
            ),
        },
        SerializableEditCommand::Replace {
            start_line,
            start_col,
            end_line,
            end_col,
            text,
        } => EditCommand::Replace {
            range: Range::new(
                Position::new(*start_line, *start_col),
                Position::new(*end_line, *end_col),
            ),
            text: text.clone(),
        },
        SerializableEditCommand::Batch(cmds) => {
            EditCommand::Batch(cmds.iter().map(from_serializable).collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn swap_file_path_derives_correctly() {
        let path = Path::new("/home/user/file.rs");
        let swap = swap_file_path(path);
        assert_eq!(swap, PathBuf::from("/home/user/.file.rs.smash-swap"));
    }

    #[test]
    fn swap_file_path_no_parent() {
        let path = Path::new("file.rs");
        let swap = swap_file_path(path);
        assert_eq!(swap, PathBuf::from(".file.rs.smash-swap"));
    }

    #[test]
    fn hash_content_deterministic() {
        let h1 = hash_content("hello world");
        let h2 = hash_content("hello world");
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_content_different_for_different_input() {
        let h1 = hash_content("hello");
        let h2 = hash_content("world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn recovery_swap_file_created_on_edit() {
        let tmp = TempDir::new().unwrap();
        let swap_path = tmp.path().join(".test.txt.smash-swap");
        let data = SwapFileData {
            original_hash: hash_content("original"),
            original_path: tmp.path().join("test.txt"),
            edits: vec![SerializableEditCommand::Insert {
                line: 0,
                col: 0,
                text: "hello".to_string(),
            }],
        };

        write_swap_file(&swap_path, &data).unwrap();
        assert!(swap_path.exists());

        let loaded = read_swap_file(&swap_path).unwrap();
        assert_eq!(loaded.original_hash, data.original_hash);
        assert_eq!(loaded.original_path, data.original_path);
        assert_eq!(loaded.edits.len(), 1);
    }

    #[test]
    fn recovery_swap_file_deleted_on_save() {
        let tmp = TempDir::new().unwrap();
        let swap_path = tmp.path().join(".test.txt.smash-swap");
        let data = SwapFileData {
            original_hash: hash_content("data"),
            original_path: tmp.path().join("test.txt"),
            edits: vec![],
        };

        write_swap_file(&swap_path, &data).unwrap();
        assert!(swap_path.exists());

        delete_swap_file(&swap_path).unwrap();
        assert!(!swap_path.exists());
    }

    #[test]
    fn recovery_replay_restores_state() {
        let tmp = TempDir::new().unwrap();
        let swap_path = tmp.path().join(".replay.txt.smash-swap");

        let cmds = [
            EditCommand::Insert {
                pos: Position::new(0, 0),
                text: "hello ".to_string(),
            },
            EditCommand::Delete {
                range: Range::new(Position::new(0, 0), Position::new(0, 6)),
            },
            EditCommand::Replace {
                range: Range::new(Position::new(1, 0), Position::new(1, 3)),
                text: "world".to_string(),
            },
        ];

        let serialized: Vec<SerializableEditCommand> = cmds.iter().map(to_serializable).collect();

        let data = SwapFileData {
            original_hash: hash_content(""),
            original_path: tmp.path().join("replay.txt"),
            edits: serialized,
        };

        write_swap_file(&swap_path, &data).unwrap();
        let loaded = read_swap_file(&swap_path).unwrap();

        let restored: Vec<EditCommand> = loaded.edits.iter().map(from_serializable).collect();

        assert_eq!(restored.len(), 3);
        assert_eq!(restored[0], cmds[0]);
        assert_eq!(restored[1], cmds[1]);
        assert_eq!(restored[2], cmds[2]);
    }

    #[test]
    fn recovery_corrupted_swap_returns_error() {
        let tmp = TempDir::new().unwrap();
        let swap_path = tmp.path().join(".bad.smash-swap");
        std::fs::write(&swap_path, b"NOT VALID JSON!!!").unwrap();

        let result = read_swap_file(&swap_path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.to_string(), "swap file corrupted");
    }

    #[test]
    fn has_swap_file_returns_false_when_missing() {
        let tmp = TempDir::new().unwrap();
        let original = tmp.path().join("nonexistent.rs");
        assert!(!has_swap_file(&original));
    }

    #[test]
    fn has_swap_file_returns_true_when_present() {
        let tmp = TempDir::new().unwrap();
        let original = tmp.path().join("exists.rs");
        let swap = swap_file_path(&original);
        let data = SwapFileData {
            original_hash: hash_content(""),
            original_path: original.clone(),
            edits: vec![],
        };
        write_swap_file(&swap, &data).unwrap();
        assert!(has_swap_file(&original));
    }

    #[test]
    fn serializable_editcommand_roundtrip() {
        let insert = EditCommand::Insert {
            pos: Position::new(5, 10),
            text: "fn main()".to_string(),
        };
        let delete = EditCommand::Delete {
            range: Range::new(Position::new(0, 0), Position::new(0, 5)),
        };
        let replace = EditCommand::Replace {
            range: Range::new(Position::new(2, 0), Position::new(2, 8)),
            text: "replaced".to_string(),
        };
        let batch = EditCommand::Batch(vec![insert.clone(), delete.clone()]);

        for cmd in &[insert, delete, replace, batch] {
            let ser = to_serializable(cmd);
            let de = from_serializable(&ser);
            assert_eq!(&de, cmd);
        }
    }

    #[test]
    fn serializable_editcommand_json_roundtrip() {
        let cmd = SerializableEditCommand::Replace {
            start_line: 1,
            start_col: 2,
            end_line: 3,
            end_col: 4,
            text: "foo".to_string(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let restored: SerializableEditCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, cmd);
    }

    #[test]
    fn delete_swap_file_noop_when_missing() {
        let tmp = TempDir::new().unwrap();
        let swap_path = tmp.path().join(".missing.smash-swap");
        assert!(delete_swap_file(&swap_path).is_ok());
    }

    #[test]
    fn indent_lines_serializable_roundtrip() {
        let cmd = EditCommand::IndentLines {
            lines: vec![0, 1, 2],
            direction: IndentDirection::In,
        };
        let ser = to_serializable(&cmd);
        // IndentLines becomes a Batch of inserts
        match &ser {
            SerializableEditCommand::Batch(cmds) => assert_eq!(cmds.len(), 3),
            other => panic!("expected Batch, got {other:?}"),
        }
    }
}
