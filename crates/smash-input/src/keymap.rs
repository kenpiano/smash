use std::collections::HashMap;

use crate::command::Command;
use crate::event::KeyEvent;

pub type KeySequence = Vec<KeyEvent>;

/// A layer name (e.g., "default", "vim-normal")
pub type LayerName = String;

/// A single layer of keybindings
#[derive(Debug, Clone, Default)]
pub struct KeymapLayer {
    name: LayerName,
    bindings: HashMap<KeySequence, Command>,
}

impl KeymapLayer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            bindings: HashMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn bind(&mut self, seq: KeySequence, cmd: Command) {
        self.bindings.insert(seq, cmd);
    }

    pub fn get(&self, seq: &[KeyEvent]) -> Option<&Command> {
        self.bindings.get(seq)
    }

    pub fn has_prefix(&self, prefix: &[KeyEvent]) -> bool {
        self.bindings
            .keys()
            .any(|seq| seq.starts_with(prefix) && seq.len() > prefix.len())
    }

    pub fn bindings(&self) -> &HashMap<KeySequence, Command> {
        &self.bindings
    }
}

/// Keymap: a stack of layers. Top layer has priority.
#[derive(Debug, Clone)]
pub struct Keymap {
    layers: Vec<KeymapLayer>,
}

impl Keymap {
    pub fn new(base: KeymapLayer) -> Self {
        Self { layers: vec![base] }
    }

    pub fn push_layer(&mut self, layer: KeymapLayer) {
        self.layers.push(layer);
    }

    pub fn pop_layer(&mut self) -> Option<KeymapLayer> {
        if self.layers.len() > 1 {
            self.layers.pop()
        } else {
            None
        }
    }

    pub fn resolve(&self, seq: &[KeyEvent]) -> Option<&Command> {
        for layer in self.layers.iter().rev() {
            if let Some(cmd) = layer.get(seq) {
                return Some(cmd);
            }
        }
        None
    }

    pub fn has_prefix(&self, prefix: &[KeyEvent]) -> bool {
        self.layers.iter().any(|l| l.has_prefix(prefix))
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::KeyEvent;

    #[test]
    fn layer_bind_and_get_returns_command() {
        let mut layer = KeymapLayer::new("test");
        let seq = vec![KeyEvent::ctrl('s')];
        layer.bind(seq.clone(), Command::Save);
        assert_eq!(layer.get(&seq), Some(&Command::Save));
    }

    #[test]
    fn layer_get_missing_key_returns_none() {
        let layer = KeymapLayer::new("test");
        let seq = vec![KeyEvent::ctrl('s')];
        assert_eq!(layer.get(&seq), None);
    }

    #[test]
    fn layer_name_returns_correct_name() {
        let layer = KeymapLayer::new("my-layer");
        assert_eq!(layer.name(), "my-layer");
    }

    #[test]
    fn layer_has_prefix_true_for_partial_match() {
        let mut layer = KeymapLayer::new("test");
        // Bind a two-key chord: Ctrl-K, Ctrl-S
        let seq = vec![KeyEvent::ctrl('k'), KeyEvent::ctrl('s')];
        layer.bind(seq, Command::Save);

        // The first key alone should be a prefix
        let prefix = vec![KeyEvent::ctrl('k')];
        assert!(layer.has_prefix(&prefix));
    }

    #[test]
    fn layer_has_prefix_false_for_exact_match() {
        let mut layer = KeymapLayer::new("test");
        let seq = vec![KeyEvent::ctrl('s')];
        layer.bind(seq.clone(), Command::Save);

        // Exact match is not a prefix
        assert!(!layer.has_prefix(&seq));
    }

    #[test]
    fn layer_has_prefix_false_when_no_match() {
        let mut layer = KeymapLayer::new("test");
        let seq = vec![KeyEvent::ctrl('s')];
        layer.bind(seq, Command::Save);

        let prefix = vec![KeyEvent::ctrl('z')];
        assert!(!layer.has_prefix(&prefix));
    }

    #[test]
    fn layer_bindings_returns_all_bindings() {
        let mut layer = KeymapLayer::new("test");
        layer.bind(vec![KeyEvent::ctrl('s')], Command::Save);
        layer.bind(vec![KeyEvent::ctrl('q')], Command::Quit);
        assert_eq!(layer.bindings().len(), 2);
    }

    #[test]
    fn keymap_resolve_from_top_layer_first() {
        let mut base = KeymapLayer::new("base");
        base.bind(vec![KeyEvent::ctrl('s')], Command::Save);

        let mut overlay = KeymapLayer::new("overlay");
        overlay.bind(vec![KeyEvent::ctrl('s')], Command::Noop);

        let mut keymap = Keymap::new(base);
        keymap.push_layer(overlay);

        let result = keymap.resolve(&[KeyEvent::ctrl('s')]);
        assert_eq!(result, Some(&Command::Noop));
    }

    #[test]
    fn keymap_resolve_falls_through_to_base() {
        let mut base = KeymapLayer::new("base");
        base.bind(vec![KeyEvent::ctrl('s')], Command::Save);

        let overlay = KeymapLayer::new("overlay");

        let mut keymap = Keymap::new(base);
        keymap.push_layer(overlay);

        let result = keymap.resolve(&[KeyEvent::ctrl('s')]);
        assert_eq!(result, Some(&Command::Save));
    }

    #[test]
    fn keymap_resolve_returns_none_when_no_binding() {
        let base = KeymapLayer::new("base");
        let keymap = Keymap::new(base);

        let result = keymap.resolve(&[KeyEvent::ctrl('z')]);
        assert_eq!(result, None);
    }

    #[test]
    fn keymap_push_pop_layer() {
        let base = KeymapLayer::new("base");
        let mut keymap = Keymap::new(base);
        assert_eq!(keymap.layer_count(), 1);

        keymap.push_layer(KeymapLayer::new("overlay"));
        assert_eq!(keymap.layer_count(), 2);

        let popped = keymap.pop_layer();
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().name(), "overlay");
        assert_eq!(keymap.layer_count(), 1);
    }

    #[test]
    fn keymap_cannot_pop_last_layer() {
        let base = KeymapLayer::new("base");
        let mut keymap = Keymap::new(base);

        let result = keymap.pop_layer();
        assert!(result.is_none());
        assert_eq!(keymap.layer_count(), 1);
    }

    #[test]
    fn keymap_has_prefix_across_layers() {
        let mut base = KeymapLayer::new("base");
        let seq = vec![KeyEvent::ctrl('k'), KeyEvent::ctrl('s')];
        base.bind(seq, Command::Save);

        let keymap = Keymap::new(base);
        let prefix = vec![KeyEvent::ctrl('k')];
        assert!(keymap.has_prefix(&prefix));
    }

    #[test]
    fn keymap_new_starts_with_one_layer() {
        let base = KeymapLayer::new("base");
        let keymap = Keymap::new(base);
        assert_eq!(keymap.layer_count(), 1);
    }

    #[test]
    fn layer_bind_overwrites_existing() {
        let mut layer = KeymapLayer::new("test");
        let seq = vec![KeyEvent::ctrl('s')];
        layer.bind(seq.clone(), Command::Save);
        layer.bind(seq.clone(), Command::Quit);
        assert_eq!(layer.get(&seq), Some(&Command::Quit));
    }

    #[test]
    fn layer_default_is_empty() {
        let layer = KeymapLayer::default();
        assert_eq!(layer.name(), "");
        assert!(layer.bindings().is_empty());
    }
}
