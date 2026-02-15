use std::fmt;
use std::time::Instant;

/// Severity level for messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageLevel {
    /// Informational message.
    Info,
    /// Warning message.
    Warning,
    /// Error message.
    Error,
}

impl fmt::Display for MessageLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageLevel::Info => write!(f, "INFO"),
            MessageLevel::Warning => write!(f, "WARN"),
            MessageLevel::Error => write!(f, "ERROR"),
        }
    }
}

/// A single message entry in the message buffer.
#[derive(Debug, Clone)]
pub struct Message {
    level: MessageLevel,
    text: String,
    timestamp: Instant,
}

impl Message {
    /// Create a new message with the current timestamp.
    pub fn new(level: MessageLevel, text: impl Into<String>) -> Self {
        Self {
            level,
            text: text.into(),
            timestamp: Instant::now(),
        }
    }

    /// The severity level of this message.
    pub fn level(&self) -> MessageLevel {
        self.level
    }

    /// The message text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The timestamp when this message was created.
    pub fn timestamp(&self) -> Instant {
        self.timestamp
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.level, self.text)
    }
}

/// A buffer dedicated to collecting editor messages and log output.
///
/// Unlike a file `Buffer`, the `MessageBuffer` is append-only and is
/// not backed by a file path. It holds status messages, warnings, and
/// errors that the editor produces during operation.
#[derive(Debug)]
pub struct MessageBuffer {
    messages: Vec<Message>,
    max_messages: usize,
}

impl Default for MessageBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageBuffer {
    /// Default maximum number of messages retained.
    const DEFAULT_MAX_MESSAGES: usize = 10_000;

    /// Create a new, empty message buffer.
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            max_messages: Self::DEFAULT_MAX_MESSAGES,
        }
    }

    /// Create a message buffer with a custom capacity limit.
    pub fn with_max_messages(max_messages: usize) -> Self {
        Self {
            messages: Vec::new(),
            max_messages,
        }
    }

    /// Append an informational message.
    pub fn info(&mut self, text: impl Into<String>) {
        self.push(MessageLevel::Info, text);
    }

    /// Append a warning message.
    pub fn warn(&mut self, text: impl Into<String>) {
        self.push(MessageLevel::Warning, text);
    }

    /// Append an error message.
    pub fn error(&mut self, text: impl Into<String>) {
        self.push(MessageLevel::Error, text);
    }

    /// Append a message with the given level.
    pub fn push(&mut self, level: MessageLevel, text: impl Into<String>) {
        let msg = Message::new(level, text);
        self.messages.push(msg);
        self.prune();
    }

    /// Number of messages currently stored.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Whether the message buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Get a message by index (0-based, oldest first).
    pub fn get(&self, index: usize) -> Option<&Message> {
        self.messages.get(index)
    }

    /// Iterate over all messages in chronological order.
    pub fn iter(&self) -> impl Iterator<Item = &Message> {
        self.messages.iter()
    }

    /// Get only messages at or above the given level.
    pub fn filter_by_level(&self, min_level: MessageLevel) -> Vec<&Message> {
        let min_ord = level_ordinal(min_level);
        self.messages
            .iter()
            .filter(|m| level_ordinal(m.level) >= min_ord)
            .collect()
    }

    /// Get the most recent message, if any.
    pub fn last(&self) -> Option<&Message> {
        self.messages.last()
    }

    /// Clear all messages.
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// The maximum number of messages this buffer retains.
    pub fn max_messages(&self) -> usize {
        self.max_messages
    }

    /// Render all messages as a single string (one per line).
    pub fn to_display_string(&self) -> String {
        self.messages
            .iter()
            .map(|m| m.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Prune oldest messages if we exceed the limit.
    fn prune(&mut self) {
        if self.messages.len() > self.max_messages {
            let excess = self.messages.len() - self.max_messages;
            self.messages.drain(..excess);
        }
    }
}

/// Map message levels to an ordinal for comparison.
fn level_ordinal(level: MessageLevel) -> u8 {
    match level {
        MessageLevel::Info => 0,
        MessageLevel::Warning => 1,
        MessageLevel::Error => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_buffer_new_is_empty() {
        let buf = MessageBuffer::new();
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
        assert!(buf.last().is_none());
    }

    #[test]
    fn message_buffer_push_info() {
        let mut buf = MessageBuffer::new();
        buf.info("file opened");
        assert_eq!(buf.len(), 1);
        let msg = buf.get(0).unwrap();
        assert_eq!(msg.level(), MessageLevel::Info);
        assert_eq!(msg.text(), "file opened");
    }

    #[test]
    fn message_buffer_push_warning() {
        let mut buf = MessageBuffer::new();
        buf.warn("trailing whitespace detected");
        assert_eq!(buf.len(), 1);
        assert_eq!(buf.get(0).unwrap().level(), MessageLevel::Warning);
    }

    #[test]
    fn message_buffer_push_error() {
        let mut buf = MessageBuffer::new();
        buf.error("file save failed");
        assert_eq!(buf.len(), 1);
        assert_eq!(buf.get(0).unwrap().level(), MessageLevel::Error);
    }

    #[test]
    fn message_buffer_multiple_messages_in_order() {
        let mut buf = MessageBuffer::new();
        buf.info("first");
        buf.warn("second");
        buf.error("third");

        assert_eq!(buf.len(), 3);
        assert_eq!(buf.get(0).unwrap().text(), "first");
        assert_eq!(buf.get(1).unwrap().text(), "second");
        assert_eq!(buf.get(2).unwrap().text(), "third");
    }

    #[test]
    fn message_buffer_last_returns_most_recent() {
        let mut buf = MessageBuffer::new();
        buf.info("old");
        buf.info("new");
        assert_eq!(buf.last().unwrap().text(), "new");
    }

    #[test]
    fn message_buffer_clear_removes_all() {
        let mut buf = MessageBuffer::new();
        buf.info("a");
        buf.info("b");
        buf.clear();
        assert!(buf.is_empty());
    }

    #[test]
    fn message_buffer_prunes_oldest_when_over_limit() {
        let mut buf = MessageBuffer::with_max_messages(3);
        buf.info("1");
        buf.info("2");
        buf.info("3");
        buf.info("4"); // should prune "1"

        assert_eq!(buf.len(), 3);
        assert_eq!(buf.get(0).unwrap().text(), "2");
        assert_eq!(buf.get(1).unwrap().text(), "3");
        assert_eq!(buf.get(2).unwrap().text(), "4");
    }

    #[test]
    fn message_buffer_prunes_multiple_excess() {
        let mut buf = MessageBuffer::with_max_messages(2);
        buf.info("a");
        buf.info("b");
        buf.info("c");
        buf.info("d");

        assert_eq!(buf.len(), 2);
        assert_eq!(buf.get(0).unwrap().text(), "c");
        assert_eq!(buf.get(1).unwrap().text(), "d");
    }

    #[test]
    fn message_buffer_filter_by_level_info() {
        let mut buf = MessageBuffer::new();
        buf.info("i");
        buf.warn("w");
        buf.error("e");

        let filtered = buf.filter_by_level(MessageLevel::Info);
        assert_eq!(filtered.len(), 3); // all messages are >= Info
    }

    #[test]
    fn message_buffer_filter_by_level_warning() {
        let mut buf = MessageBuffer::new();
        buf.info("i");
        buf.warn("w");
        buf.error("e");

        let filtered = buf.filter_by_level(MessageLevel::Warning);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].text(), "w");
        assert_eq!(filtered[1].text(), "e");
    }

    #[test]
    fn message_buffer_filter_by_level_error() {
        let mut buf = MessageBuffer::new();
        buf.info("i");
        buf.warn("w");
        buf.error("e");

        let filtered = buf.filter_by_level(MessageLevel::Error);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].text(), "e");
    }

    #[test]
    fn message_buffer_iter_yields_all_messages() {
        let mut buf = MessageBuffer::new();
        buf.info("a");
        buf.warn("b");

        let texts: Vec<&str> = buf.iter().map(|m| m.text()).collect();
        assert_eq!(texts, vec!["a", "b"]);
    }

    #[test]
    fn message_display_format() {
        let msg = Message::new(MessageLevel::Info, "hello");
        assert_eq!(msg.to_string(), "[INFO] hello");

        let msg = Message::new(MessageLevel::Warning, "careful");
        assert_eq!(msg.to_string(), "[WARN] careful");

        let msg = Message::new(MessageLevel::Error, "boom");
        assert_eq!(msg.to_string(), "[ERROR] boom");
    }

    #[test]
    fn message_buffer_to_display_string() {
        let mut buf = MessageBuffer::new();
        buf.info("line one");
        buf.error("line two");

        let output = buf.to_display_string();
        assert_eq!(output, "[INFO] line one\n[ERROR] line two");
    }

    #[test]
    fn message_buffer_default_max_messages() {
        let buf = MessageBuffer::new();
        assert_eq!(buf.max_messages(), 10_000);
    }

    #[test]
    fn message_buffer_get_out_of_bounds_returns_none() {
        let buf = MessageBuffer::new();
        assert!(buf.get(0).is_none());
        assert!(buf.get(100).is_none());
    }

    #[test]
    fn message_buffer_default_trait() {
        let buf = MessageBuffer::default();
        assert!(buf.is_empty());
        assert_eq!(buf.max_messages(), 10_000);
    }

    #[test]
    fn message_level_display() {
        assert_eq!(MessageLevel::Info.to_string(), "INFO");
        assert_eq!(MessageLevel::Warning.to_string(), "WARN");
        assert_eq!(MessageLevel::Error.to_string(), "ERROR");
    }

    #[test]
    fn message_timestamp_is_recent() {
        let before = Instant::now();
        let msg = Message::new(MessageLevel::Info, "test");
        let after = Instant::now();

        assert!(msg.timestamp() >= before);
        assert!(msg.timestamp() <= after);
    }

    #[test]
    fn message_buffer_push_with_level() {
        let mut buf = MessageBuffer::new();
        buf.push(MessageLevel::Warning, "custom push");
        assert_eq!(buf.len(), 1);
        assert_eq!(buf.get(0).unwrap().level(), MessageLevel::Warning);
        assert_eq!(buf.get(0).unwrap().text(), "custom push");
    }

    #[test]
    fn message_buffer_is_not_file_buffer() {
        // MessageBuffer has no path, no rope, no undo — it's a
        // completely separate type from Buffer.
        let mut msg_buf = MessageBuffer::new();
        msg_buf.info("editor started");
        msg_buf.warn("unsaved changes");

        // Verify it stores messages, not editable text
        assert_eq!(msg_buf.len(), 2);
        assert_eq!(msg_buf.get(0).unwrap().text(), "editor started");
        assert_eq!(msg_buf.get(1).unwrap().text(), "unsaved changes");

        // Cannot confuse with a file buffer — different type entirely
        let file_buf = super::super::buffer::Buffer::new(super::super::buffer::BufferId::next());
        assert!(file_buf.is_empty());
        assert!(file_buf.path().is_none());
        // File buffer has no messages API; message buffer has no edit API
    }
}
