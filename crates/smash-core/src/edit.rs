use crate::position::{Position, Range};

/// Direction for indentation operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndentDirection {
    In,
    Out,
}

/// Case transformation variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaseTransform {
    Upper,
    Lower,
    Title,
}

/// A command describing a single edit operation on a buffer.
#[derive(Debug, Clone, PartialEq)]
pub enum EditCommand {
    Insert {
        pos: Position,
        text: String,
    },
    Delete {
        range: Range,
    },
    Replace {
        range: Range,
        text: String,
    },
    IndentLines {
        lines: Vec<usize>,
        direction: IndentDirection,
    },
    Batch(Vec<EditCommand>),
}

/// Event emitted after a successful edit, for subscribers
/// (syntax, LSP, collab).
#[derive(Debug, Clone)]
pub struct EditEvent {
    pub start_byte: usize,
    pub old_end_byte: usize,
    pub new_end_byte: usize,
    pub start_position: Position,
    pub old_end_position: Position,
    pub new_end_position: Position,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_command_insert_construction() {
        let cmd = EditCommand::Insert {
            pos: Position::new(0, 0),
            text: "hello".to_string(),
        };
        match &cmd {
            EditCommand::Insert { pos, text } => {
                assert_eq!(*pos, Position::new(0, 0));
                assert_eq!(text, "hello");
            }
            _ => panic!("expected Insert"),
        }
    }

    #[test]
    fn edit_command_delete_construction() {
        let cmd = EditCommand::Delete {
            range: Range::new(Position::new(0, 0), Position::new(0, 5)),
        };
        match &cmd {
            EditCommand::Delete { range } => {
                assert_eq!(range.start, Position::new(0, 0));
                assert_eq!(range.end, Position::new(0, 5));
            }
            _ => panic!("expected Delete"),
        }
    }

    #[test]
    fn edit_command_replace_construction() {
        let cmd = EditCommand::Replace {
            range: Range::new(Position::new(1, 0), Position::new(1, 3)),
            text: "xyz".to_string(),
        };
        match &cmd {
            EditCommand::Replace { range, text } => {
                assert_eq!(range.start, Position::new(1, 0));
                assert_eq!(text, "xyz");
            }
            _ => panic!("expected Replace"),
        }
    }

    #[test]
    fn edit_command_indent_lines_construction() {
        let cmd = EditCommand::IndentLines {
            lines: vec![0, 1, 2],
            direction: IndentDirection::In,
        };
        match &cmd {
            EditCommand::IndentLines { lines, direction } => {
                assert_eq!(lines, &[0, 1, 2]);
                assert_eq!(*direction, IndentDirection::In);
            }
            _ => panic!("expected IndentLines"),
        }
    }

    #[test]
    fn edit_command_batch_construction() {
        let cmds = vec![
            EditCommand::Insert {
                pos: Position::new(0, 0),
                text: "a".to_string(),
            },
            EditCommand::Delete {
                range: Range::new(Position::new(1, 0), Position::new(1, 1)),
            },
        ];
        let batch = EditCommand::Batch(cmds.clone());
        match &batch {
            EditCommand::Batch(inner) => assert_eq!(inner.len(), 2),
            _ => panic!("expected Batch"),
        }
    }

    #[test]
    fn edit_command_clone_and_eq() {
        let cmd = EditCommand::Insert {
            pos: Position::new(0, 0),
            text: "test".to_string(),
        };
        let cmd2 = cmd.clone();
        assert_eq!(cmd, cmd2);
    }

    #[test]
    fn indent_direction_eq() {
        assert_eq!(IndentDirection::In, IndentDirection::In);
        assert_ne!(IndentDirection::In, IndentDirection::Out);
    }

    #[test]
    fn case_transform_eq() {
        assert_eq!(CaseTransform::Upper, CaseTransform::Upper);
        assert_ne!(CaseTransform::Upper, CaseTransform::Lower);
        assert_ne!(CaseTransform::Lower, CaseTransform::Title);
    }

    #[test]
    fn edit_event_debug() {
        let evt = EditEvent {
            start_byte: 0,
            old_end_byte: 5,
            new_end_byte: 10,
            start_position: Position::new(0, 0),
            old_end_position: Position::new(0, 5),
            new_end_position: Position::new(0, 10),
        };
        // Ensure Debug is implemented
        let _ = format!("{evt:?}");
    }
}
