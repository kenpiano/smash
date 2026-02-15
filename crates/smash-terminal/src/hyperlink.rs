use regex::Regex;

/// A detected hyperlink in the terminal grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedLink {
    /// Row in the grid (0-based).
    pub row: u16,
    /// Start column (inclusive, 0-based).
    pub start_col: u16,
    /// End column (exclusive, 0-based).
    pub end_col: u16,
    /// The URI or path string.
    pub uri: String,
    /// Whether this is an explicit OSC 8 hyperlink.
    pub is_osc8: bool,
}

/// Scans terminal grid rows for URLs and file paths.
#[derive(Debug)]
pub struct HyperlinkDetector {
    url_regex: Regex,
    path_regex: Regex,
}

impl HyperlinkDetector {
    /// Create a new hyperlink detector.
    pub fn new() -> Self {
        // Match common URLs: http(s), ftp, file
        let url_regex =
            Regex::new(r#"https?://[^\s<>"')\]]+|ftp://[^\s<>"')\]]+|file://[^\s<>"')\]]+"#)
                .expect("URL regex is valid");

        // Match absolute file paths with optional line number
        // e.g., /home/user/file.rs:10 or /home/user/file.rs:10:5
        let path_regex = Regex::new(r"/[a-zA-Z0-9_\-./]+\.[a-zA-Z0-9]+(?::\d+(?::\d+)?)?")
            .expect("Path regex is valid");

        Self {
            url_regex,
            path_regex,
        }
    }

    /// Detect hyperlinks in a single row of text.
    /// Returns a list of detected links.
    pub fn detect_in_text(&self, row: u16, text: &str) -> Vec<DetectedLink> {
        let mut links = Vec::new();

        // Detect URLs
        for mat in self.url_regex.find_iter(text) {
            links.push(DetectedLink {
                row,
                start_col: mat.start() as u16,
                end_col: mat.end() as u16,
                uri: mat.as_str().to_string(),
                is_osc8: false,
            });
        }

        // Detect file paths (only if not already covered by a URL match)
        for mat in self.path_regex.find_iter(text) {
            let start = mat.start() as u16;
            let end = mat.end() as u16;
            let overlaps = links
                .iter()
                .any(|l| l.row == row && start < l.end_col && end > l.start_col);
            if !overlaps {
                links.push(DetectedLink {
                    row,
                    start_col: start,
                    end_col: end,
                    uri: mat.as_str().to_string(),
                    is_osc8: false,
                });
            }
        }

        links
    }

    /// Detect hyperlinks across all rows in the grid.
    pub fn detect_in_grid(&self, grid: &crate::grid::TerminalGrid) -> Vec<DetectedLink> {
        let mut all_links = Vec::new();
        let rows = grid.size().rows;

        for row in 0..rows {
            let text = grid.row_text(row);
            let mut row_links = self.detect_in_text(row, &text);
            all_links.append(&mut row_links);
        }

        all_links
    }
}

impl Default for HyperlinkDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Create an explicit OSC 8 hyperlink annotation.
pub fn osc8_link(row: u16, start_col: u16, end_col: u16, uri: &str) -> DetectedLink {
    DetectedLink {
        row,
        start_col,
        end_col,
        uri: uri.to_string(),
        is_osc8: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hyperlink_detect_url() {
        let detector = HyperlinkDetector::new();
        let links = detector.detect_in_text(0, "Visit https://example.com for info");

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].uri, "https://example.com");
        assert_eq!(links[0].start_col, 6);
        assert_eq!(links[0].end_col, 25);
        assert!(!links[0].is_osc8);
    }

    #[test]
    fn hyperlink_detect_file_path() {
        let detector = HyperlinkDetector::new();
        let links = detector.detect_in_text(0, "Error at /home/user/file.rs:10");

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].uri, "/home/user/file.rs:10");
        assert!(!links[0].is_osc8);
    }

    #[test]
    fn hyperlink_no_false_positive() {
        let detector = HyperlinkDetector::new();
        let links = detector.detect_in_text(0, "not a link at all");

        assert!(links.is_empty());
    }

    #[test]
    fn hyperlink_osc8_explicit() {
        let link = osc8_link(0, 5, 15, "https://rust-lang.org");
        assert!(link.is_osc8);
        assert_eq!(link.uri, "https://rust-lang.org");
        assert_eq!(link.start_col, 5);
        assert_eq!(link.end_col, 15);
    }

    #[test]
    fn hyperlink_detect_multiple_urls() {
        let detector = HyperlinkDetector::new();
        let links = detector.detect_in_text(0, "See https://a.com and https://b.com please");
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].uri, "https://a.com");
        assert_eq!(links[1].uri, "https://b.com");
    }

    #[test]
    fn hyperlink_detect_ftp() {
        let detector = HyperlinkDetector::new();
        let links = detector.detect_in_text(0, "Download from ftp://files.example.com/data.tar");
        assert_eq!(links.len(), 1);
        assert!(links[0].uri.starts_with("ftp://"));
    }

    #[test]
    fn hyperlink_detect_path_with_line_and_col() {
        let detector = HyperlinkDetector::new();
        let links = detector.detect_in_text(0, "Error at /src/main.rs:42:10");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].uri, "/src/main.rs:42:10");
    }

    #[test]
    fn hyperlink_detect_in_grid() {
        let detector = HyperlinkDetector::new();
        let mut grid = crate::grid::TerminalGrid::new(80, 3);
        // Write a URL on row 1
        grid.cursor_set_position(1, 0);
        for ch in "https://example.com".chars() {
            grid.write_char(ch);
        }

        let links = detector.detect_in_grid(&grid);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].row, 1);
        assert_eq!(links[0].uri, "https://example.com");
    }
}
