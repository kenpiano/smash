use std::time::Instant;

use crate::edit::EditCommand;
use crate::position::Position;

/// Maximum number of undo nodes before pruning.
const MAX_UNDO_NODES: usize = 10_000;

/// A node in the undo tree.
#[derive(Debug, Clone)]
struct UndoNode {
    /// The operation that undoes this edit (moves to parent state).
    backward: EditCommand,
    /// The original edit (for redo — moves to child state).
    forward: EditCommand,
    /// Cursor position before the edit was applied.
    cursor_before: Position,
    /// Indices of child nodes in the arena.
    children: Vec<usize>,
    /// Index of the parent node (None for root).
    parent: Option<usize>,
    /// When this edit was recorded.
    #[allow(dead_code)]
    timestamp: Instant,
}

/// An arena-based undo tree.
///
/// The tree starts with a sentinel root node (index 0). Edits are
/// recorded as children of the current node, and undo/redo navigates
/// up/down the tree.
#[derive(Debug)]
pub struct UndoTree {
    nodes: Vec<UndoNode>,
    current: usize,
}

impl UndoTree {
    /// Create a new undo tree with a sentinel root.
    pub fn new() -> Self {
        let root = UndoNode {
            backward: EditCommand::Batch(Vec::new()),
            forward: EditCommand::Batch(Vec::new()),
            cursor_before: Position::default(),
            children: Vec::new(),
            parent: None,
            timestamp: Instant::now(),
        };
        Self {
            nodes: vec![root],
            current: 0,
        }
    }

    /// Record an edit. `backward` is the inverse (undo) op,
    /// `forward` is the original edit (redo) op.
    pub fn record(&mut self, backward: EditCommand, forward: EditCommand, cursor_before: Position) {
        let new_idx = self.nodes.len();
        let node = UndoNode {
            backward,
            forward,
            cursor_before,
            children: Vec::new(),
            parent: Some(self.current),
            timestamp: Instant::now(),
        };
        self.nodes.push(node);
        self.nodes[self.current].children.push(new_idx);
        self.current = new_idx;

        if self.nodes.len() > MAX_UNDO_NODES {
            self.prune();
        }
    }

    /// Undo: move to parent, returning the backward (undo) operation
    /// and the cursor position before that edit.
    pub fn undo(&mut self) -> Option<(EditCommand, Position)> {
        if self.current == 0 {
            return None;
        }
        let node = &self.nodes[self.current];
        let result = (node.backward.clone(), node.cursor_before);
        if let Some(parent) = node.parent {
            self.current = parent;
        }
        Some(result)
    }

    /// Redo: move to the last child of the current node,
    /// returning the forward (redo) operation.
    pub fn redo(&mut self) -> Option<(EditCommand, Position)> {
        let children = &self.nodes[self.current].children;
        if children.is_empty() {
            return None;
        }
        // Pick the last child (most recent branch)
        let child_idx = *children.last()?;
        self.current = child_idx;
        let node = &self.nodes[self.current];
        Some((node.forward.clone(), node.cursor_before))
    }

    /// Whether undo is possible (we are not at root).
    pub fn can_undo(&self) -> bool {
        self.current != 0
    }

    /// Whether redo is possible (current node has children).
    pub fn can_redo(&self) -> bool {
        !self.nodes[self.current].children.is_empty()
    }

    /// Total number of nodes including root.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the tree is empty (only root).
    pub fn is_empty(&self) -> bool {
        self.nodes.len() <= 1
    }

    /// Prune when the tree exceeds the limit.
    /// Phase 1: remove off-path leaf nodes.
    /// Phase 2: collapse old nodes on the current path near root.
    fn prune(&mut self) {
        // Build the set of nodes on the current path
        let mut path_set = vec![false; self.nodes.len()];
        let mut idx = self.current;
        loop {
            path_set[idx] = true;
            match self.nodes[idx].parent {
                Some(p) => idx = p,
                None => break,
            }
        }

        // Phase 1: remove off-path leaf nodes
        while self.nodes.len() > MAX_UNDO_NODES {
            let found =
                (1..self.nodes.len()).find(|&i| !path_set[i] && self.nodes[i].children.is_empty());
            match found {
                Some(leaf) => {
                    self.remove_node(leaf, &mut path_set);
                }
                None => break,
            }
        }

        // Phase 2: collapse old path nodes near root
        while self.nodes.len() > MAX_UNDO_NODES {
            let path_child = self.nodes[0]
                .children
                .iter()
                .copied()
                .find(|&c| path_set.get(c).copied().unwrap_or(false));
            match path_child {
                Some(child) if child != self.current => {
                    let grandchildren = self.nodes[child].children.clone();
                    self.nodes[0].children.retain(|&c| c != child);
                    for &gc in &grandchildren {
                        self.nodes[0].children.push(gc);
                        self.nodes[gc].parent = Some(0);
                    }
                    self.nodes[child].children.clear();
                    if child < path_set.len() {
                        path_set[child] = false;
                    }
                    self.remove_node(child, &mut path_set);
                }
                _ => break,
            }
        }
    }

    /// Remove a leaf node and clean up parent references.
    fn remove_node(&mut self, idx: usize, path_set: &mut [bool]) {
        if let Some(parent) = self.nodes[idx].parent {
            self.nodes[parent].children.retain(|&c| c != idx);
        }

        // Swap-remove the node
        let last = self.nodes.len() - 1;
        if idx != last {
            self.nodes.swap(idx, last);
            path_set.swap(idx, last);

            // Fix references to the moved node (was at `last`, now at `idx`)
            if let Some(parent) = self.nodes[idx].parent {
                for c in &mut self.nodes[parent].children {
                    if *c == last {
                        *c = idx;
                    }
                }
            }
            for child in self.nodes[idx].children.clone() {
                if child < self.nodes.len() {
                    self.nodes[child].parent = Some(idx);
                }
            }

            // Fix current pointer
            if self.current == last {
                self.current = idx;
            }
        }
        self.nodes.pop();
    }
}

impl Default for UndoTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::Range;

    fn insert_cmd(text: &str) -> EditCommand {
        EditCommand::Insert {
            pos: Position::new(0, 0),
            text: text.to_string(),
        }
    }

    fn delete_cmd() -> EditCommand {
        EditCommand::Delete {
            range: Range::new(Position::new(0, 0), Position::new(0, 1)),
        }
    }

    #[test]
    fn new_tree_is_empty() {
        let tree = UndoTree::new();
        assert!(tree.is_empty());
        assert!(!tree.can_undo());
        assert!(!tree.can_redo());
        assert_eq!(tree.len(), 1); // root only
    }

    #[test]
    fn record_and_undo() {
        let mut tree = UndoTree::new();
        // backward=insert (undo), forward=delete (redo)
        tree.record(insert_cmd("a"), delete_cmd(), Position::new(0, 0));
        assert!(tree.can_undo());
        assert_eq!(tree.len(), 2);

        let (cmd, cursor) = tree.undo().unwrap();
        assert_eq!(cursor, Position::new(0, 0));
        match cmd {
            EditCommand::Insert { text, .. } => {
                assert_eq!(text, "a");
            }
            _ => panic!("expected Insert"),
        }
        assert!(!tree.can_undo());
    }

    #[test]
    fn undo_at_root_returns_none() {
        let mut tree = UndoTree::new();
        assert!(tree.undo().is_none());
    }

    #[test]
    fn record_and_redo() {
        let mut tree = UndoTree::new();
        tree.record(delete_cmd(), insert_cmd("a"), Position::new(0, 0));
        tree.undo();
        assert!(tree.can_redo());

        // redo returns the forward op
        let (cmd, _) = tree.redo().unwrap();
        match cmd {
            EditCommand::Insert { text, .. } => {
                assert_eq!(text, "a");
            }
            _ => panic!("expected Insert"),
        }
    }

    #[test]
    fn redo_at_leaf_returns_none() {
        let mut tree = UndoTree::new();
        tree.record(delete_cmd(), insert_cmd("a"), Position::new(0, 0));
        assert!(tree.redo().is_none());
    }

    #[test]
    fn branching_undo_then_new_edit() {
        let mut tree = UndoTree::new();
        tree.record(delete_cmd(), insert_cmd("a"), Position::new(0, 0));
        tree.record(delete_cmd(), insert_cmd("b"), Position::new(0, 1));

        // Undo back to after "a"
        tree.undo();
        assert!(tree.can_redo());

        // New edit creates a branch
        tree.record(delete_cmd(), insert_cmd("c"), Position::new(0, 1));

        // Redo goes to the latest branch ("c")
        tree.undo();
        let (cmd, _) = tree.redo().unwrap();
        match cmd {
            EditCommand::Insert { text, .. } => {
                assert_eq!(text, "c");
            }
            _ => panic!("expected Insert"),
        }
    }

    #[test]
    fn multiple_undo_redo() {
        let mut tree = UndoTree::new();
        tree.record(delete_cmd(), insert_cmd("1"), Position::new(0, 0));
        tree.record(delete_cmd(), insert_cmd("2"), Position::new(0, 1));
        tree.record(delete_cmd(), insert_cmd("3"), Position::new(0, 2));

        // Undo all
        assert!(tree.undo().is_some());
        assert!(tree.undo().is_some());
        assert!(tree.undo().is_some());
        assert!(tree.undo().is_none());

        // Redo all
        assert!(tree.redo().is_some());
        assert!(tree.redo().is_some());
        assert!(tree.redo().is_some());
        assert!(tree.redo().is_none());
    }

    #[test]
    fn len_tracks_nodes() {
        let mut tree = UndoTree::new();
        assert_eq!(tree.len(), 1);
        tree.record(delete_cmd(), insert_cmd("x"), Position::new(0, 0));
        assert_eq!(tree.len(), 2);
        tree.record(delete_cmd(), insert_cmd("y"), Position::new(0, 0));
        assert_eq!(tree.len(), 3);
    }

    #[test]
    fn pruning_at_limit() {
        let mut tree = UndoTree::new();
        // Record more than MAX_UNDO_NODES entries
        for i in 0..MAX_UNDO_NODES + 100 {
            tree.record(
                delete_cmd(),
                insert_cmd(&format!("{i}")),
                Position::new(0, 0),
            );
        }
        // Should have been pruned to at most MAX_UNDO_NODES
        assert!(tree.len() <= MAX_UNDO_NODES + 1);
        // Current node should still be valid
        assert!(tree.can_undo());
    }

    #[test]
    fn is_empty_after_record_is_false() {
        let mut tree = UndoTree::new();
        tree.record(delete_cmd(), insert_cmd("x"), Position::new(0, 0));
        assert!(!tree.is_empty());
    }
}
