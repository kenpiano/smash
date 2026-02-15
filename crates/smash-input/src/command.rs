#[derive(Debug, Clone, PartialEq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    // Buffer ops
    InsertChar(char),
    InsertNewline,
    DeleteBackward,
    DeleteForward,
    DeleteLine,
    // Cursor movement
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    MoveWordLeft,
    MoveWordRight,
    MoveLineStart,
    MoveLineEnd,
    MoveBufferStart,
    MoveBufferEnd,
    PageUp,
    PageDown,
    // Selection
    SelectAll,
    ExtendSelection(Direction),
    AddCursorAbove,
    AddCursorBelow,
    // File ops
    Save,
    SaveAs,
    Open,
    Close,
    // Search
    Find,
    FindReplace,
    FindNext,
    FindPrev,
    // Undo
    Undo,
    Redo,
    // Panes
    SplitVertical,
    SplitHorizontal,
    FocusNext,
    FocusPrev,
    ClosePane,
    // Nav
    GoToLine,
    OpenCommandPalette,
    OpenFileFinder,
    // Terminal
    ToggleTerminal,
    NewTerminal,
    // LSP
    LspHover,
    LspGotoDefinition,
    LspFindReferences,
    LspCompletion,
    LspFormat,
    LspRename,
    LspCodeAction,
    LspDiagnosticNext,
    LspDiagnosticPrev,
    LspRestart,
    // Lifecycle
    Quit,
    ForceQuit,
    // Noop (for unbound keys)
    Noop,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_insert_char_holds_value() {
        let cmd = Command::InsertChar('x');
        assert_eq!(cmd, Command::InsertChar('x'));
    }

    #[test]
    fn command_direction_variants_exist() {
        let _ = Direction::Left;
        let _ = Direction::Right;
        let _ = Direction::Up;
        let _ = Direction::Down;
    }

    #[test]
    fn command_extend_selection_holds_direction() {
        let cmd = Command::ExtendSelection(Direction::Right);
        assert_eq!(cmd, Command::ExtendSelection(Direction::Right));
    }

    #[test]
    fn command_clone_equals_original() {
        let cmd = Command::Save;
        let cloned = cmd.clone();
        assert_eq!(cmd, cloned);
    }

    #[test]
    fn command_noop_variant_exists() {
        let cmd = Command::Noop;
        assert_eq!(cmd, Command::Noop);
    }

    #[test]
    fn command_debug_formatting() {
        let cmd = Command::Quit;
        let debug = format!("{:?}", cmd);
        assert_eq!(debug, "Quit");
    }

    #[test]
    fn command_all_movement_variants() {
        let movements = vec![
            Command::MoveLeft,
            Command::MoveRight,
            Command::MoveUp,
            Command::MoveDown,
            Command::MoveWordLeft,
            Command::MoveWordRight,
            Command::MoveLineStart,
            Command::MoveLineEnd,
            Command::MoveBufferStart,
            Command::MoveBufferEnd,
            Command::PageUp,
            Command::PageDown,
        ];
        assert_eq!(movements.len(), 12);
    }

    #[test]
    fn command_all_file_ops() {
        let ops = [
            Command::Save,
            Command::SaveAs,
            Command::Open,
            Command::Close,
        ];
        assert_eq!(ops.len(), 4);
    }
}
