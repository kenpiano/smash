/// Represents the line-ending convention used in a text buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum LineEnding {
    #[default]
    Lf,
    CrLf,
    Cr,
}

impl LineEnding {
    /// Return the string representation of this line ending.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
            Self::Cr => "\r",
        }
    }
}

/// Detect the dominant line ending in `text` by counting occurrences.
///
/// Scans the text for `\r\n`, `\n` (standalone), and `\r` (standalone).
/// Returns the most common variant, defaulting to `Lf` for empty text
/// or ties.
pub fn detect_line_ending(text: &str) -> LineEnding {
    let mut crlf_count: usize = 0;
    let mut lf_count: usize = 0;
    let mut cr_count: usize = 0;

    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'\r' {
            if i + 1 < len && bytes[i + 1] == b'\n' {
                crlf_count += 1;
                i += 2;
            } else {
                cr_count += 1;
                i += 1;
            }
        } else if bytes[i] == b'\n' {
            lf_count += 1;
            i += 1;
        } else {
            i += 1;
        }
    }

    if crlf_count == 0 && lf_count == 0 && cr_count == 0 {
        return LineEnding::default();
    }

    if crlf_count >= lf_count && crlf_count >= cr_count {
        LineEnding::CrLf
    } else if lf_count >= cr_count {
        LineEnding::Lf
    } else {
        LineEnding::Cr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_ending_as_str() {
        assert_eq!(LineEnding::Lf.as_str(), "\n");
        assert_eq!(LineEnding::CrLf.as_str(), "\r\n");
        assert_eq!(LineEnding::Cr.as_str(), "\r");
    }

    #[test]
    fn line_ending_default_is_lf() {
        assert_eq!(LineEnding::default(), LineEnding::Lf);
    }

    #[test]
    fn detect_empty_text() {
        assert_eq!(detect_line_ending(""), LineEnding::Lf);
    }

    #[test]
    fn detect_lf_only() {
        let text = "hello\nworld\nfoo\n";
        assert_eq!(detect_line_ending(text), LineEnding::Lf);
    }

    #[test]
    fn detect_crlf_only() {
        let text = "hello\r\nworld\r\nfoo\r\n";
        assert_eq!(detect_line_ending(text), LineEnding::CrLf);
    }

    #[test]
    fn detect_cr_only() {
        let text = "hello\rworld\rfoo\r";
        assert_eq!(detect_line_ending(text), LineEnding::Cr);
    }

    #[test]
    fn detect_mixed_majority_lf() {
        // 3 LF, 1 CRLF, 1 CR → LF wins
        let text = "a\nb\nc\nd\r\ne\r";
        assert_eq!(detect_line_ending(text), LineEnding::Lf);
    }

    #[test]
    fn detect_mixed_majority_crlf() {
        // 1 LF, 3 CRLF → CRLF wins
        let text = "a\r\nb\r\nc\r\nd\n";
        assert_eq!(detect_line_ending(text), LineEnding::CrLf);
    }

    #[test]
    fn detect_mixed_majority_cr() {
        // 3 CR, 1 LF → CR wins
        let text = "a\rb\rc\rd\n";
        assert_eq!(detect_line_ending(text), LineEnding::Cr);
    }

    #[test]
    fn detect_no_line_endings() {
        assert_eq!(detect_line_ending("no newlines here"), LineEnding::Lf);
    }

    #[test]
    fn detect_single_lf() {
        assert_eq!(detect_line_ending("\n"), LineEnding::Lf);
    }

    #[test]
    fn detect_single_crlf() {
        assert_eq!(detect_line_ending("\r\n"), LineEnding::CrLf);
    }

    #[test]
    fn detect_single_cr() {
        assert_eq!(detect_line_ending("\r"), LineEnding::Cr);
    }
}
