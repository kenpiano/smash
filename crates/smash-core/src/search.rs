use crate::position::{Position, Range};

/// A search query: either a plain-text pattern or a regex.
#[derive(Debug, Clone)]
pub enum SearchQuery {
    Plain {
        pattern: String,
        case_sensitive: bool,
    },
    Regex(regex::Regex),
}

impl SearchQuery {
    /// Find all matches of this query in `text`.
    pub fn find_all(&self, text: &str) -> Vec<SearchMatch> {
        match self {
            SearchQuery::Plain {
                pattern,
                case_sensitive,
            } => find_plain(text, pattern, *case_sensitive),
            SearchQuery::Regex(re) => find_regex(text, re),
        }
    }
}

/// A single search match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
    pub range: Range,
    pub byte_range: std::ops::Range<usize>,
}

/// Stateful search context for a buffer.
#[derive(Debug)]
pub struct SearchState {
    query: Option<SearchQuery>,
    matches: Vec<SearchMatch>,
    current_index: Option<usize>,
}

impl SearchState {
    /// Create an empty search state.
    pub fn new() -> Self {
        Self {
            query: None,
            matches: Vec::new(),
            current_index: None,
        }
    }

    /// Set a query and immediately search `text`.
    pub fn set_query(&mut self, query: SearchQuery, text: &str) {
        let matches = query.find_all(text);
        self.query = Some(query);
        self.matches = matches;
        self.current_index = if self.matches.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    /// Advance to the next match (wrapping around).
    pub fn next_match(&mut self) -> Option<&SearchMatch> {
        if self.matches.is_empty() {
            return None;
        }
        let idx = match self.current_index {
            Some(i) => (i + 1) % self.matches.len(),
            None => 0,
        };
        self.current_index = Some(idx);
        Some(&self.matches[idx])
    }

    /// Go to the previous match (wrapping around).
    pub fn prev_match(&mut self) -> Option<&SearchMatch> {
        if self.matches.is_empty() {
            return None;
        }
        let idx = match self.current_index {
            Some(0) => self.matches.len() - 1,
            Some(i) => i - 1,
            None => self.matches.len() - 1,
        };
        self.current_index = Some(idx);
        Some(&self.matches[idx])
    }

    /// Number of matches found.
    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    /// Clear the search state.
    pub fn clear(&mut self) {
        self.query = None;
        self.matches.clear();
        self.current_index = None;
    }
}

impl Default for SearchState {
    fn default() -> Self {
        Self::new()
    }
}

// --- Private helpers ---

/// Convert a byte offset into (line, col) position within `text`.
fn byte_offset_to_position(text: &str, byte_offset: usize) -> Position {
    let mut line = 0;
    let mut col = 0;
    for (i, ch) in text.char_indices() {
        if i == byte_offset {
            return Position::new(line, col);
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    // byte_offset == text.len() means the position after the last char
    Position::new(line, col)
}

fn find_plain(text: &str, pattern: &str, case_sensitive: bool) -> Vec<SearchMatch> {
    if pattern.is_empty() {
        return Vec::new();
    }

    let mut results = Vec::new();

    if case_sensitive {
        let mut start = 0;
        while let Some(idx) = text[start..].find(pattern) {
            let byte_start = start + idx;
            let byte_end = byte_start + pattern.len();
            let start_pos = byte_offset_to_position(text, byte_start);
            let end_pos = byte_offset_to_position(text, byte_end);
            results.push(SearchMatch {
                range: Range::new(start_pos, end_pos),
                byte_range: byte_start..byte_end,
            });
            start = byte_start + 1;
        }
    } else {
        let lower_text = text.to_lowercase();
        let lower_pattern = pattern.to_lowercase();
        let mut start = 0;
        while let Some(idx) = lower_text[start..].find(&lower_pattern) {
            let byte_start = start + idx;
            let byte_end = byte_start + lower_pattern.len();
            let start_pos = byte_offset_to_position(text, byte_start);
            let end_pos = byte_offset_to_position(text, byte_end);
            results.push(SearchMatch {
                range: Range::new(start_pos, end_pos),
                byte_range: byte_start..byte_end,
            });
            start = byte_start + 1;
        }
    }

    results
}

fn find_regex(text: &str, re: &regex::Regex) -> Vec<SearchMatch> {
    re.find_iter(text)
        .map(|m| {
            let start_pos = byte_offset_to_position(text, m.start());
            let end_pos = byte_offset_to_position(text, m.end());
            SearchMatch {
                range: Range::new(start_pos, end_pos),
                byte_range: m.start()..m.end(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_case_sensitive_finds_exact() {
        let q = SearchQuery::Plain {
            pattern: "foo".to_string(),
            case_sensitive: true,
        };
        let matches = q.find_all("foo bar foo");
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].byte_range, 0..3);
        assert_eq!(matches[1].byte_range, 8..11);
    }

    #[test]
    fn plain_case_sensitive_no_match() {
        let q = SearchQuery::Plain {
            pattern: "FOO".to_string(),
            case_sensitive: true,
        };
        let matches = q.find_all("foo bar");
        assert!(matches.is_empty());
    }

    #[test]
    fn plain_case_insensitive_finds_all() {
        let q = SearchQuery::Plain {
            pattern: "foo".to_string(),
            case_sensitive: false,
        };
        let matches = q.find_all("FOO fOo foo");
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn plain_empty_pattern_no_matches() {
        let q = SearchQuery::Plain {
            pattern: String::new(),
            case_sensitive: true,
        };
        assert!(q.find_all("some text").is_empty());
    }

    #[test]
    fn regex_search_finds_matches() {
        let re = regex::Regex::new(r"\d+").unwrap();
        let q = SearchQuery::Regex(re);
        let matches = q.find_all("abc 123 def 456");
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].byte_range, 4..7);
        assert_eq!(matches[1].byte_range, 12..15);
    }

    #[test]
    fn regex_search_no_match() {
        let re = regex::Regex::new(r"\d+").unwrap();
        let q = SearchQuery::Regex(re);
        assert!(q.find_all("no digits here").is_empty());
    }

    #[test]
    fn search_match_positions_multiline() {
        let text = "line0\nline1\nfoo";
        let q = SearchQuery::Plain {
            pattern: "foo".to_string(),
            case_sensitive: true,
        };
        let matches = q.find_all(text);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].range.start, Position::new(2, 0));
        assert_eq!(matches[0].range.end, Position::new(2, 3));
    }

    #[test]
    fn search_state_new_is_empty() {
        let ss = SearchState::new();
        assert_eq!(ss.match_count(), 0);
    }

    #[test]
    fn search_state_set_query_finds_matches() {
        let mut ss = SearchState::new();
        ss.set_query(
            SearchQuery::Plain {
                pattern: "ab".to_string(),
                case_sensitive: true,
            },
            "ab cd ab",
        );
        assert_eq!(ss.match_count(), 2);
    }

    #[test]
    fn search_state_next_wraps_around() {
        let mut ss = SearchState::new();
        ss.set_query(
            SearchQuery::Plain {
                pattern: "x".to_string(),
                case_sensitive: true,
            },
            "x y x",
        );
        assert_eq!(ss.match_count(), 2);

        // Initial index is 0, next goes to 1
        let m1 = ss.next_match().unwrap().clone();
        assert_eq!(m1.byte_range, 4..5); // second match

        // next wraps to 0
        let m2 = ss.next_match().unwrap().clone();
        assert_eq!(m2.byte_range, 0..1);
    }

    #[test]
    fn search_state_prev_wraps_around() {
        let mut ss = SearchState::new();
        ss.set_query(
            SearchQuery::Plain {
                pattern: "z".to_string(),
                case_sensitive: true,
            },
            "z a z",
        );
        assert_eq!(ss.match_count(), 2);

        // Initial index is 0, prev wraps to last
        let m = ss.prev_match().unwrap().clone();
        assert_eq!(m.byte_range, 4..5);
    }

    #[test]
    fn search_state_next_empty_returns_none() {
        let mut ss = SearchState::new();
        assert!(ss.next_match().is_none());
    }

    #[test]
    fn search_state_prev_empty_returns_none() {
        let mut ss = SearchState::new();
        assert!(ss.prev_match().is_none());
    }

    #[test]
    fn search_state_clear() {
        let mut ss = SearchState::new();
        ss.set_query(
            SearchQuery::Plain {
                pattern: "a".to_string(),
                case_sensitive: true,
            },
            "aaa",
        );
        assert_eq!(ss.match_count(), 3);
        ss.clear();
        assert_eq!(ss.match_count(), 0);
        assert!(ss.next_match().is_none());
    }

    #[test]
    fn byte_offset_to_position_basic() {
        let text = "ab\ncd\nef";
        assert_eq!(byte_offset_to_position(text, 0), Position::new(0, 0));
        assert_eq!(byte_offset_to_position(text, 2), Position::new(0, 2));
        assert_eq!(byte_offset_to_position(text, 3), Position::new(1, 0));
        assert_eq!(byte_offset_to_position(text, 6), Position::new(2, 0));
        assert_eq!(byte_offset_to_position(text, 8), Position::new(2, 2));
    }
}
