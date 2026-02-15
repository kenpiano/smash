use crate::error::TuiError;

/// Rectangle in terminal coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn area(&self) -> u32 {
        self.width as u32 * self.height as u32
    }

    pub fn contains(&self, col: u16, row: u16) -> bool {
        col >= self.x && col < self.x + self.width && row >= self.y && row < self.y + self.height
    }
}

pub type PaneId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

/// A node in the pane tree: either a split or a leaf.
#[derive(Debug)]
enum PaneNode {
    Leaf {
        id: PaneId,
    },
    Split {
        direction: SplitDirection,
        /// Ratio of first child (0.0 to 1.0).
        ratio: f64,
        first: Box<PaneNode>,
        second: Box<PaneNode>,
    },
}

/// Manages the pane tree layout.
#[derive(Debug)]
pub struct PaneTree {
    root: PaneNode,
    next_id: PaneId,
    active: PaneId,
}

impl PaneTree {
    pub fn new() -> Self {
        Self {
            root: PaneNode::Leaf { id: 0 },
            next_id: 1,
            active: 0,
        }
    }

    pub fn active_pane(&self) -> PaneId {
        self.active
    }

    pub fn set_active(&mut self, id: PaneId) {
        self.active = id;
    }

    /// Split the active pane, returns new pane's ID.
    pub fn split(&mut self, direction: SplitDirection) -> Result<PaneId, TuiError> {
        let new_id = self.next_id;
        self.next_id += 1;
        let target = self.active;
        let replaced = Self::split_node(&mut self.root, target, direction, new_id);
        if replaced {
            Ok(new_id)
        } else {
            Err(TuiError::Layout(format!("pane {} not found", target)))
        }
    }

    fn split_node(
        node: &mut PaneNode,
        target: PaneId,
        direction: SplitDirection,
        new_id: PaneId,
    ) -> bool {
        match node {
            PaneNode::Leaf { id } if *id == target => {
                let old_leaf = PaneNode::Leaf { id: *id };
                let new_leaf = PaneNode::Leaf { id: new_id };
                *node = PaneNode::Split {
                    direction,
                    ratio: 0.5,
                    first: Box::new(old_leaf),
                    second: Box::new(new_leaf),
                };
                true
            }
            PaneNode::Split { first, second, .. } => {
                Self::split_node(first, target, direction, new_id)
                    || Self::split_node(second, target, direction, new_id)
            }
            _ => false,
        }
    }

    /// Close a pane. Returns Err if it's the last pane.
    pub fn close(&mut self, id: PaneId) -> Result<(), TuiError> {
        if matches!(self.root, PaneNode::Leaf { .. }) {
            return Err(TuiError::Layout("cannot close the last pane".into()));
        }
        let closed = Self::close_node(&mut self.root, id);
        if closed {
            // If active pane was closed, find a new active
            if self.active == id {
                self.active = Self::first_leaf(&self.root);
            }
            Ok(())
        } else {
            Err(TuiError::Layout(format!("pane {} not found", id)))
        }
    }

    fn close_node(node: &mut PaneNode, id: PaneId) -> bool {
        match node {
            PaneNode::Split { first, second, .. } => {
                if matches!(
                    **first,
                    PaneNode::Leaf { id: lid } if lid == id
                ) {
                    let taken = std::mem::replace(second.as_mut(), PaneNode::Leaf { id: 0 });
                    *node = taken;
                    return true;
                }
                if matches!(
                    **second,
                    PaneNode::Leaf { id: lid } if lid == id
                ) {
                    let taken = std::mem::replace(first.as_mut(), PaneNode::Leaf { id: 0 });
                    *node = taken;
                    return true;
                }
                // Recurse
                Self::close_node(first, id) || Self::close_node(second, id)
            }
            _ => false,
        }
    }

    fn first_leaf(node: &PaneNode) -> PaneId {
        match node {
            PaneNode::Leaf { id } => *id,
            PaneNode::Split { first, .. } => Self::first_leaf(first),
        }
    }

    /// Compute the layout rectangles for all panes.
    pub fn layout(&self, area: Rect) -> Vec<(PaneId, Rect)> {
        let mut result = Vec::new();
        Self::layout_node(&self.root, area, &mut result);
        result
    }

    fn layout_node(node: &PaneNode, area: Rect, result: &mut Vec<(PaneId, Rect)>) {
        match node {
            PaneNode::Leaf { id } => {
                result.push((*id, area));
            }
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let (a, b) = split_rect(area, *direction, *ratio);
                Self::layout_node(first, a, result);
                Self::layout_node(second, b, result);
            }
        }
    }

    /// Get all leaf pane IDs.
    pub fn pane_ids(&self) -> Vec<PaneId> {
        let mut ids = Vec::new();
        Self::collect_ids(&self.root, &mut ids);
        ids
    }

    fn collect_ids(node: &PaneNode, ids: &mut Vec<PaneId>) {
        match node {
            PaneNode::Leaf { id } => ids.push(*id),
            PaneNode::Split { first, second, .. } => {
                Self::collect_ids(first, ids);
                Self::collect_ids(second, ids);
            }
        }
    }

    /// Cycle focus to next pane.
    pub fn focus_next(&mut self) {
        let ids = self.pane_ids();
        if let Some(pos) = ids.iter().position(|&id| id == self.active) {
            self.active = ids[(pos + 1) % ids.len()];
        }
    }

    /// Cycle focus to previous pane.
    pub fn focus_prev(&mut self) {
        let ids = self.pane_ids();
        if let Some(pos) = ids.iter().position(|&id| id == self.active) {
            self.active = ids[(pos + ids.len() - 1) % ids.len()];
        }
    }
}

impl Default for PaneTree {
    fn default() -> Self {
        Self::new()
    }
}

fn split_rect(area: Rect, direction: SplitDirection, ratio: f64) -> (Rect, Rect) {
    match direction {
        SplitDirection::Vertical => {
            let left_w = (area.width as f64 * ratio) as u16;
            let right_w = area.width.saturating_sub(left_w);
            (
                Rect::new(area.x, area.y, left_w, area.height),
                Rect::new(area.x + left_w, area.y, right_w, area.height),
            )
        }
        SplitDirection::Horizontal => {
            let top_h = (area.height as f64 * ratio) as u16;
            let bot_h = area.height.saturating_sub(top_h);
            (
                Rect::new(area.x, area.y, area.width, top_h),
                Rect::new(area.x, area.y + top_h, area.width, bot_h),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_new_and_fields() {
        let r = Rect::new(1, 2, 80, 24);
        assert_eq!(r.x, 1);
        assert_eq!(r.y, 2);
        assert_eq!(r.width, 80);
        assert_eq!(r.height, 24);
    }

    #[test]
    fn rect_area() {
        let r = Rect::new(0, 0, 80, 24);
        assert_eq!(r.area(), 1920);
    }

    #[test]
    fn rect_contains_inside() {
        let r = Rect::new(10, 10, 20, 10);
        assert!(r.contains(10, 10));
        assert!(r.contains(29, 19));
        assert!(r.contains(15, 15));
    }

    #[test]
    fn rect_contains_outside() {
        let r = Rect::new(10, 10, 20, 10);
        assert!(!r.contains(9, 10));
        assert!(!r.contains(10, 9));
        assert!(!r.contains(30, 10));
        assert!(!r.contains(10, 20));
    }

    #[test]
    fn pane_tree_new_single_pane() {
        let tree = PaneTree::new();
        assert_eq!(tree.active_pane(), 0);
        assert_eq!(tree.pane_ids(), vec![0]);
    }

    #[test]
    fn pane_tree_default_same_as_new() {
        let tree = PaneTree::default();
        assert_eq!(tree.active_pane(), 0);
        assert_eq!(tree.pane_ids(), vec![0]);
    }

    #[test]
    fn pane_tree_split_creates_two_panes() {
        let mut tree = PaneTree::new();
        let new_id = tree.split(SplitDirection::Vertical).unwrap();
        assert_eq!(new_id, 1);
        let ids = tree.pane_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&0));
        assert!(ids.contains(&1));
    }

    #[test]
    fn pane_tree_split_vertical_layout_halves_width() {
        let mut tree = PaneTree::new();
        tree.split(SplitDirection::Vertical).unwrap();
        let area = Rect::new(0, 0, 80, 24);
        let layout = tree.layout(area);
        assert_eq!(layout.len(), 2);
        let (_, r0) = layout[0];
        let (_, r1) = layout[1];
        assert_eq!(r0.width, 40);
        assert_eq!(r1.width, 40);
        assert_eq!(r0.height, 24);
        assert_eq!(r1.height, 24);
    }

    #[test]
    fn pane_tree_split_horizontal_layout_halves_height() {
        let mut tree = PaneTree::new();
        tree.split(SplitDirection::Horizontal).unwrap();
        let area = Rect::new(0, 0, 80, 24);
        let layout = tree.layout(area);
        assert_eq!(layout.len(), 2);
        let (_, r0) = layout[0];
        let (_, r1) = layout[1];
        assert_eq!(r0.width, 80);
        assert_eq!(r1.width, 80);
        assert_eq!(r0.height, 12);
        assert_eq!(r1.height, 12);
    }

    #[test]
    fn pane_tree_close_pane() {
        let mut tree = PaneTree::new();
        let new_id = tree.split(SplitDirection::Vertical).unwrap();
        tree.close(new_id).unwrap();
        assert_eq!(tree.pane_ids(), vec![0]);
    }

    #[test]
    fn pane_tree_close_last_pane_fails() {
        let mut tree = PaneTree::new();
        let result = tree.close(0);
        assert!(result.is_err());
    }

    #[test]
    fn pane_tree_close_active_pane_updates_active() {
        let mut tree = PaneTree::new();
        tree.split(SplitDirection::Vertical).unwrap();
        tree.close(0).unwrap();
        assert_eq!(tree.active_pane(), 1);
    }

    #[test]
    fn pane_tree_focus_next_cycles() {
        let mut tree = PaneTree::new();
        tree.split(SplitDirection::Vertical).unwrap();
        assert_eq!(tree.active_pane(), 0);
        tree.focus_next();
        assert_eq!(tree.active_pane(), 1);
        tree.focus_next();
        assert_eq!(tree.active_pane(), 0);
    }

    #[test]
    fn pane_tree_focus_prev_cycles() {
        let mut tree = PaneTree::new();
        tree.split(SplitDirection::Vertical).unwrap();
        assert_eq!(tree.active_pane(), 0);
        tree.focus_prev();
        assert_eq!(tree.active_pane(), 1);
        tree.focus_prev();
        assert_eq!(tree.active_pane(), 0);
    }

    #[test]
    fn pane_tree_set_active() {
        let mut tree = PaneTree::new();
        tree.split(SplitDirection::Vertical).unwrap();
        tree.set_active(1);
        assert_eq!(tree.active_pane(), 1);
    }

    #[test]
    fn pane_tree_nested_splits() {
        let mut tree = PaneTree::new();
        tree.split(SplitDirection::Vertical).unwrap();
        // Split pane 0 horizontally
        tree.set_active(0);
        tree.split(SplitDirection::Horizontal).unwrap();
        let ids = tree.pane_ids();
        assert_eq!(ids.len(), 3);
        let area = Rect::new(0, 0, 80, 24);
        let layout = tree.layout(area);
        assert_eq!(layout.len(), 3);
    }

    #[test]
    fn split_rect_vertical() {
        let area = Rect::new(0, 0, 100, 50);
        let (a, b) = split_rect(area, SplitDirection::Vertical, 0.5);
        assert_eq!(a.width, 50);
        assert_eq!(b.width, 50);
        assert_eq!(a.x, 0);
        assert_eq!(b.x, 50);
    }

    #[test]
    fn split_rect_horizontal() {
        let area = Rect::new(0, 0, 100, 50);
        let (a, b) = split_rect(area, SplitDirection::Horizontal, 0.5);
        assert_eq!(a.height, 25);
        assert_eq!(b.height, 25);
        assert_eq!(a.y, 0);
        assert_eq!(b.y, 25);
    }

    #[test]
    fn pane_tree_layout_single_pane_fills_area() {
        let tree = PaneTree::new();
        let area = Rect::new(0, 0, 80, 24);
        let layout = tree.layout(area);
        assert_eq!(layout.len(), 1);
        assert_eq!(layout[0], (0, area));
    }

    #[test]
    fn pane_tree_close_nonexistent_pane_fails() {
        let mut tree = PaneTree::new();
        tree.split(SplitDirection::Vertical).unwrap();
        let result = tree.close(99);
        assert!(result.is_err());
    }
}
