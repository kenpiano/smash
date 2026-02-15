use regex::Regex;

use crate::error::SyntaxError;
use crate::highlight::{HighlightEngine, HighlightSpan};
use crate::language::LanguageId;
use crate::scope::ScopeId;

/// A single highlighting rule: a regex pattern + scope.
struct HighlightRule {
    regex: Regex,
    scope: ScopeId,
}

/// Regex-based syntax highlighter for a specific language.
///
/// Rules are evaluated in order; earlier rules have higher
/// priority. Overlapping matches from later rules are
/// discarded.
pub struct RegexHighlighter {
    language: LanguageId,
    rules: Vec<HighlightRule>,
}

impl RegexHighlighter {
    /// Create a new highlighter for the given language.
    pub fn new(language: LanguageId) -> Result<Self, SyntaxError> {
        let rules = build_rules(language)?;
        Ok(Self { language, rules })
    }

    /// The language this highlighter was built for.
    pub fn language(&self) -> LanguageId {
        self.language
    }
}

impl HighlightEngine for RegexHighlighter {
    fn highlight_line(&self, line: &str) -> Vec<HighlightSpan> {
        let mut spans: Vec<HighlightSpan> = Vec::new();

        for rule in &self.rules {
            for mat in rule.regex.find_iter(line) {
                let span = HighlightSpan::new(mat.start(), mat.end(), rule.scope);
                // Only add if it doesn't overlap existing
                if !overlaps(&spans, &span) {
                    spans.push(span);
                }
            }
        }

        spans.sort_by_key(|s| s.start);
        spans
    }

    fn engine_name(&self) -> &str {
        "regex"
    }
}

/// Returns true if `new` overlaps with any span in `existing`.
fn overlaps(existing: &[HighlightSpan], new: &HighlightSpan) -> bool {
    existing
        .iter()
        .any(|s| s.start < new.end && new.start < s.end)
}

fn build_rules(lang: LanguageId) -> Result<Vec<HighlightRule>, SyntaxError> {
    match lang {
        LanguageId::Rust => rust_rules(),
        LanguageId::Python => python_rules(),
        LanguageId::JavaScript | LanguageId::TypeScript => js_rules(),
        LanguageId::C | LanguageId::Cpp => c_rules(),
        LanguageId::Go => go_rules(),
        LanguageId::Toml => toml_rules(),
        LanguageId::Json => json_rules(),
        LanguageId::Shell => shell_rules(),
        // Plain / Markdown = no highlighting rules
        _ => Ok(Vec::new()),
    }
}

fn make_rule(pattern: &str, scope: ScopeId, lang: &str) -> Result<HighlightRule, SyntaxError> {
    let regex = Regex::new(pattern).map_err(|e| SyntaxError::InvalidPattern {
        language: lang.to_string(),
        detail: e.to_string(),
    })?;
    Ok(HighlightRule { regex, scope })
}

fn rust_rules() -> Result<Vec<HighlightRule>, SyntaxError> {
    let lang = "rust";
    Ok(vec![
        // Comments first (highest priority)
        make_rule(r"//[^\n]*", ScopeId::Comment, lang)?,
        // Strings
        make_rule(r#""(?:[^"\\]|\\.)*""#, ScopeId::String, lang)?,
        // Char literals
        make_rule(r"'(?:[^'\\]|\\.)'", ScopeId::String, lang)?,
        // Numbers
        make_rule(
            concat!(
                r"\b\d[\d_]*",
                r"(?:\.\d[\d_]*)?",
                r"(?:[eE][+-]?\d+)?",
                r"(?:f32|f64|i8|i16|i32|i64|i128",
                r"|u8|u16|u32|u64|u128|usize|isize)?\b",
            ),
            ScopeId::Number,
            lang,
        )?,
        make_rule(r"\b0x[0-9a-fA-F_]+\b", ScopeId::Number, lang)?,
        make_rule(r"\b0b[01_]+\b", ScopeId::Number, lang)?,
        // Macros (word followed by !)
        make_rule(r"\b[a-z_]\w*!", ScopeId::Macro, lang)?,
        // Keywords
        make_rule(
            concat!(
                r"\b(?:as|async|await|break|const|continue",
                r"|crate|dyn|else|enum|extern|fn|for|if",
                r"|impl|in|let|loop|match|mod|move|mut",
                r"|pub|ref|return|self|Self|static|struct",
                r"|super|trait|type|unsafe|use|where",
                r"|while|yield)\b",
            ),
            ScopeId::Keyword,
            lang,
        )?,
        // Types
        make_rule(
            concat!(
                r"\b(?:bool|char|f32|f64|i8|i16|i32|i64",
                r"|i128|isize|str|u8|u16|u32|u64|u128",
                r"|usize|String|Vec|Option|Result|Box|Rc",
                r"|Arc|HashMap|HashSet|BTreeMap",
                r"|BTreeSet)\b",
            ),
            ScopeId::Type,
            lang,
        )?,
        // Constants
        make_rule(
            r"\b(?:true|false|None|Some|Ok|Err)\b",
            ScopeId::Constant,
            lang,
        )?,
        // Attributes
        make_rule(r"#\[[\w:(),= ]*\]", ScopeId::Attribute, lang)?,
        // Operators
        make_rule(r"[+\-*/=<>!&|^%]+", ScopeId::Operator, lang)?,
    ])
}

fn python_rules() -> Result<Vec<HighlightRule>, SyntaxError> {
    let lang = "python";
    Ok(vec![
        make_rule(r"#[^\n]*", ScopeId::Comment, lang)?,
        make_rule(r#""""[\s\S]*?""""#, ScopeId::String, lang)?,
        make_rule(r#""(?:[^"\\]|\\.)*""#, ScopeId::String, lang)?,
        make_rule(r"'(?:[^'\\]|\\.)*'", ScopeId::String, lang)?,
        make_rule(r"\b\d[\d_]*(?:\.\d[\d_]*)?\b", ScopeId::Number, lang)?,
        make_rule(
            concat!(
                r"\b(?:and|as|assert|async|await|break",
                r"|class|continue|def|del|elif|else",
                r"|except|finally|for|from|global|if",
                r"|import|in|is|lambda|nonlocal|not|or",
                r"|pass|raise|return|try|while|with",
                r"|yield)\b",
            ),
            ScopeId::Keyword,
            lang,
        )?,
        make_rule(r"\b(?:True|False|None)\b", ScopeId::Constant, lang)?,
        make_rule(r"@\w+", ScopeId::Attribute, lang)?,
        make_rule(r"[+\-*/=<>!&|^%]+", ScopeId::Operator, lang)?,
    ])
}

fn js_rules() -> Result<Vec<HighlightRule>, SyntaxError> {
    let lang = "javascript";
    Ok(vec![
        make_rule(r"//[^\n]*", ScopeId::Comment, lang)?,
        make_rule(r#""(?:[^"\\]|\\.)*""#, ScopeId::String, lang)?,
        make_rule(r"'(?:[^'\\]|\\.)*'", ScopeId::String, lang)?,
        make_rule(r"`[^`]*`", ScopeId::String, lang)?,
        make_rule(r"\b\d[\d_]*(?:\.\d[\d_]*)?\b", ScopeId::Number, lang)?,
        make_rule(
            concat!(
                r"\b(?:async|await|break|case|catch|class",
                r"|const|continue|debugger|default|delete",
                r"|do|else|export|extends|finally|for",
                r"|from|function|if|import|in|instanceof",
                r"|let|new|of|return|static|super|switch",
                r"|this|throw|try|typeof|var|void|while",
                r"|with|yield)\b",
            ),
            ScopeId::Keyword,
            lang,
        )?,
        make_rule(
            concat!(r"\b(?:true|false|null|undefined", r"|NaN|Infinity)\b",),
            ScopeId::Constant,
            lang,
        )?,
        make_rule(r"[+\-*/=<>!&|^%]+", ScopeId::Operator, lang)?,
    ])
}

fn c_rules() -> Result<Vec<HighlightRule>, SyntaxError> {
    let lang = "c";
    Ok(vec![
        make_rule(r"//[^\n]*", ScopeId::Comment, lang)?,
        make_rule(r#""(?:[^"\\]|\\.)*""#, ScopeId::String, lang)?,
        make_rule(r"\b\d[\d_]*(?:\.\d[\d_]*)?\b", ScopeId::Number, lang)?,
        make_rule(r"\b0x[0-9a-fA-F]+\b", ScopeId::Number, lang)?,
        make_rule(
            concat!(
                r"#\s*(?:include|define|ifdef|ifndef",
                r"|endif|if|else|elif|undef|pragma)\b",
            ),
            ScopeId::Attribute,
            lang,
        )?,
        make_rule(
            concat!(
                r"\b(?:auto|break|case|char|const",
                r"|continue|default|do|double|else|enum",
                r"|extern|float|for|goto|if|inline|int",
                r"|long|register|return|short|signed",
                r"|sizeof|static|struct|switch|typedef",
                r"|union|unsigned|void|volatile|while)\b",
            ),
            ScopeId::Keyword,
            lang,
        )?,
        make_rule(r"\b(?:NULL|true|false)\b", ScopeId::Constant, lang)?,
        make_rule(r"[+\-*/=<>!&|^%]+", ScopeId::Operator, lang)?,
    ])
}

fn go_rules() -> Result<Vec<HighlightRule>, SyntaxError> {
    let lang = "go";
    Ok(vec![
        make_rule(r"//[^\n]*", ScopeId::Comment, lang)?,
        make_rule(r#""(?:[^"\\]|\\.)*""#, ScopeId::String, lang)?,
        make_rule(r"`[^`]*`", ScopeId::String, lang)?,
        make_rule(r"\b\d[\d_]*(?:\.\d[\d_]*)?\b", ScopeId::Number, lang)?,
        make_rule(
            concat!(
                r"\b(?:break|case|chan|const|continue",
                r"|default|defer|else|fallthrough|for",
                r"|func|go|goto|if|import|interface|map",
                r"|package|range|return|select|struct",
                r"|switch|type|var)\b",
            ),
            ScopeId::Keyword,
            lang,
        )?,
        make_rule(r"\b(?:true|false|nil|iota)\b", ScopeId::Constant, lang)?,
        make_rule(
            concat!(
                r"\b(?:bool|byte|complex64|complex128",
                r"|error|float32|float64|int|int8|int16",
                r"|int32|int64|rune|string|uint|uint8",
                r"|uint16|uint32|uint64|uintptr)\b",
            ),
            ScopeId::Type,
            lang,
        )?,
        make_rule(r"[+\-*/=<>!&|^%]+", ScopeId::Operator, lang)?,
    ])
}

fn toml_rules() -> Result<Vec<HighlightRule>, SyntaxError> {
    let lang = "toml";
    Ok(vec![
        make_rule(r"#[^\n]*", ScopeId::Comment, lang)?,
        make_rule(r#""(?:[^"\\]|\\.)*""#, ScopeId::String, lang)?,
        make_rule(r"'[^']*'", ScopeId::String, lang)?,
        make_rule(r"\b\d[\d_]*(?:\.\d[\d_]*)?\b", ScopeId::Number, lang)?,
        make_rule(r"\b(?:true|false)\b", ScopeId::Constant, lang)?,
        make_rule(r"\[[\w\.\-]+\]", ScopeId::Label, lang)?,
    ])
}

fn json_rules() -> Result<Vec<HighlightRule>, SyntaxError> {
    let lang = "json";
    Ok(vec![
        make_rule(r#""(?:[^"\\]|\\.)*"\s*:"#, ScopeId::Label, lang)?,
        make_rule(r#""(?:[^"\\]|\\.)*""#, ScopeId::String, lang)?,
        make_rule(r"\b\d[\d]*(?:\.\d[\d]*)?\b", ScopeId::Number, lang)?,
        make_rule(r"\b(?:true|false|null)\b", ScopeId::Constant, lang)?,
    ])
}

fn shell_rules() -> Result<Vec<HighlightRule>, SyntaxError> {
    let lang = "shell";
    Ok(vec![
        make_rule(r"#[^\n]*", ScopeId::Comment, lang)?,
        make_rule(r#""(?:[^"\\]|\\.)*""#, ScopeId::String, lang)?,
        make_rule(r"'[^']*'", ScopeId::String, lang)?,
        make_rule(r"\$\{?\w+\}?", ScopeId::Variable, lang)?,
        make_rule(r"\b\d+\b", ScopeId::Number, lang)?,
        make_rule(
            concat!(
                r"\b(?:if|then|else|elif|fi|for|while",
                r"|do|done|case|esac|in|function|return",
                r"|local|export|source|alias|unalias",
                r"|set|unset|readonly|shift|exit|exec",
                r"|eval|trap)\b",
            ),
            ScopeId::Keyword,
            lang,
        )?,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    // ----- construction -----

    #[test]
    fn new_rust_highlighter_succeeds() {
        let h = RegexHighlighter::new(LanguageId::Rust);
        assert!(h.is_ok());
    }

    #[test]
    fn language_accessor_returns_language() {
        let h = RegexHighlighter::new(LanguageId::Python).unwrap();
        assert_eq!(h.language(), LanguageId::Python);
    }

    #[test]
    fn engine_name_is_regex() {
        let h = RegexHighlighter::new(LanguageId::Rust).unwrap();
        assert_eq!(h.engine_name(), "regex");
    }

    // ----- Rust highlighting -----

    #[test]
    fn rust_fn_keyword_highlighted() {
        let h = RegexHighlighter::new(LanguageId::Rust).unwrap();
        let spans = h.highlight_line("fn main() {");
        let kw = spans.iter().find(|s| s.scope == ScopeId::Keyword);
        assert!(kw.is_some());
        let kw = kw.unwrap();
        assert_eq!(kw.start, 0);
        assert_eq!(kw.end, 2);
    }

    #[test]
    fn rust_string_literal_highlighted() {
        let h = RegexHighlighter::new(LanguageId::Rust).unwrap();
        let spans = h.highlight_line(r#"let s = "hello";"#);
        let string_span = spans.iter().find(|s| s.scope == ScopeId::String);
        assert!(string_span.is_some());
        let ss = string_span.unwrap();
        assert_eq!(&r#"let s = "hello";"#[ss.start..ss.end], "\"hello\"");
    }

    #[test]
    fn rust_comment_highlighted() {
        let h = RegexHighlighter::new(LanguageId::Rust).unwrap();
        let spans = h.highlight_line("// this is a comment");
        let comment = spans.iter().find(|s| s.scope == ScopeId::Comment);
        assert!(comment.is_some());
        let c = comment.unwrap();
        assert_eq!(c.start, 0);
        assert_eq!(c.end, 20);
    }

    #[test]
    fn rust_number_highlighted() {
        let h = RegexHighlighter::new(LanguageId::Rust).unwrap();
        let spans = h.highlight_line("let x = 42;");
        let num = spans.iter().find(|s| s.scope == ScopeId::Number);
        assert!(num.is_some());
        let n = num.unwrap();
        assert_eq!(&"let x = 42;"[n.start..n.end], "42");
    }

    #[test]
    fn rust_hex_number_highlighted() {
        let h = RegexHighlighter::new(LanguageId::Rust).unwrap();
        let spans = h.highlight_line("let x = 0xFF;");
        let num = spans.iter().find(|s| s.scope == ScopeId::Number);
        assert!(num.is_some());
    }

    #[test]
    fn rust_macro_highlighted() {
        let h = RegexHighlighter::new(LanguageId::Rust).unwrap();
        let spans = h.highlight_line("println!(\"hi\");");
        let mac = spans.iter().find(|s| s.scope == ScopeId::Macro);
        assert!(mac.is_some());
        let m = mac.unwrap();
        assert_eq!(&"println!(\"hi\");"[m.start..m.end], "println!");
    }

    #[test]
    fn rust_attribute_highlighted() {
        let h = RegexHighlighter::new(LanguageId::Rust).unwrap();
        let spans = h.highlight_line("#[derive(Debug)]");
        let attr = spans.iter().find(|s| s.scope == ScopeId::Attribute);
        assert!(attr.is_some());
    }

    #[test]
    fn rust_constant_highlighted() {
        let h = RegexHighlighter::new(LanguageId::Rust).unwrap();
        let spans = h.highlight_line("let b = true;");
        let c = spans.iter().find(|s| s.scope == ScopeId::Constant);
        assert!(c.is_some());
        let c = c.unwrap();
        assert_eq!(&"let b = true;"[c.start..c.end], "true");
    }

    #[test]
    fn rust_type_highlighted() {
        let h = RegexHighlighter::new(LanguageId::Rust).unwrap();
        let spans = h.highlight_line("let v: Vec<u32> = Vec::new();");
        let ty = spans.iter().find(|s| s.scope == ScopeId::Type);
        assert!(ty.is_some());
    }

    // ----- Python highlighting -----

    #[test]
    fn python_keyword_highlighted() {
        let h = RegexHighlighter::new(LanguageId::Python).unwrap();
        let spans = h.highlight_line("def foo():");
        let kw = spans.iter().find(|s| s.scope == ScopeId::Keyword);
        assert!(kw.is_some());
        let k = kw.unwrap();
        assert_eq!(&"def foo():"[k.start..k.end], "def");
    }

    #[test]
    fn python_comment_highlighted() {
        let h = RegexHighlighter::new(LanguageId::Python).unwrap();
        let spans = h.highlight_line("# comment");
        let c = spans.iter().find(|s| s.scope == ScopeId::Comment);
        assert!(c.is_some());
    }

    #[test]
    fn python_constant_highlighted() {
        let h = RegexHighlighter::new(LanguageId::Python).unwrap();
        let spans = h.highlight_line("x = True");
        let c = spans.iter().find(|s| s.scope == ScopeId::Constant);
        assert!(c.is_some());
    }

    #[test]
    fn python_decorator_highlighted() {
        let h = RegexHighlighter::new(LanguageId::Python).unwrap();
        let spans = h.highlight_line("@staticmethod");
        let a = spans.iter().find(|s| s.scope == ScopeId::Attribute);
        assert!(a.is_some());
    }

    // ----- JavaScript highlighting -----

    #[test]
    fn js_keyword_highlighted() {
        let h = RegexHighlighter::new(LanguageId::JavaScript).unwrap();
        let spans = h.highlight_line("const x = 1;");
        let kw = spans.iter().find(|s| s.scope == ScopeId::Keyword);
        assert!(kw.is_some());
        let k = kw.unwrap();
        assert_eq!(&"const x = 1;"[k.start..k.end], "const");
    }

    #[test]
    fn js_template_string_highlighted() {
        let h = RegexHighlighter::new(LanguageId::JavaScript).unwrap();
        let spans = h.highlight_line("`hello`");
        let s = spans.iter().find(|sp| sp.scope == ScopeId::String);
        assert!(s.is_some());
    }

    #[test]
    fn typescript_uses_js_rules() {
        let h = RegexHighlighter::new(LanguageId::TypeScript).unwrap();
        let spans = h.highlight_line("const x = 1;");
        let kw = spans.iter().find(|s| s.scope == ScopeId::Keyword);
        assert!(kw.is_some());
    }

    // ----- C highlighting -----

    #[test]
    fn c_preprocessor_highlighted() {
        let h = RegexHighlighter::new(LanguageId::C).unwrap();
        let spans = h.highlight_line("#include <stdio.h>");
        let attr = spans.iter().find(|s| s.scope == ScopeId::Attribute);
        assert!(attr.is_some());
    }

    #[test]
    fn cpp_uses_c_rules() {
        let h = RegexHighlighter::new(LanguageId::Cpp).unwrap();
        let spans = h.highlight_line("int main() {}");
        let kw = spans.iter().find(|s| s.scope == ScopeId::Keyword);
        assert!(kw.is_some());
    }

    // ----- Go highlighting -----

    #[test]
    fn go_keyword_highlighted() {
        let h = RegexHighlighter::new(LanguageId::Go).unwrap();
        let spans = h.highlight_line("func main() {}");
        let kw = spans.iter().find(|s| s.scope == ScopeId::Keyword);
        assert!(kw.is_some());
        let k = kw.unwrap();
        assert_eq!(&"func main() {}"[k.start..k.end], "func");
    }

    #[test]
    fn go_type_highlighted() {
        let h = RegexHighlighter::new(LanguageId::Go).unwrap();
        let spans = h.highlight_line("var x int");
        let ty = spans.iter().find(|s| s.scope == ScopeId::Type);
        assert!(ty.is_some());
    }

    // ----- TOML highlighting -----

    #[test]
    fn toml_section_highlighted() {
        let h = RegexHighlighter::new(LanguageId::Toml).unwrap();
        let spans = h.highlight_line("[package]");
        let lbl = spans.iter().find(|s| s.scope == ScopeId::Label);
        assert!(lbl.is_some());
    }

    #[test]
    fn toml_comment_highlighted() {
        let h = RegexHighlighter::new(LanguageId::Toml).unwrap();
        let spans = h.highlight_line("# a comment");
        let c = spans.iter().find(|s| s.scope == ScopeId::Comment);
        assert!(c.is_some());
    }

    // ----- JSON highlighting -----

    #[test]
    fn json_key_highlighted_as_label() {
        let h = RegexHighlighter::new(LanguageId::Json).unwrap();
        let spans = h.highlight_line(r#""name": "smash""#);
        let lbl = spans.iter().find(|s| s.scope == ScopeId::Label);
        assert!(lbl.is_some());
    }

    #[test]
    fn json_value_string_highlighted() {
        let h = RegexHighlighter::new(LanguageId::Json).unwrap();
        let spans = h.highlight_line(r#""name": "smash""#);
        let s = spans.iter().find(|sp| sp.scope == ScopeId::String);
        assert!(s.is_some());
    }

    #[test]
    fn json_constant_highlighted() {
        let h = RegexHighlighter::new(LanguageId::Json).unwrap();
        let spans = h.highlight_line(r#""active": true"#);
        let c = spans.iter().find(|s| s.scope == ScopeId::Constant);
        assert!(c.is_some());
    }

    // ----- Shell highlighting -----

    #[test]
    fn shell_variable_highlighted() {
        let h = RegexHighlighter::new(LanguageId::Shell).unwrap();
        let spans = h.highlight_line("echo $HOME");
        let var = spans.iter().find(|s| s.scope == ScopeId::Variable);
        assert!(var.is_some());
    }

    #[test]
    fn shell_keyword_highlighted() {
        let h = RegexHighlighter::new(LanguageId::Shell).unwrap();
        let spans = h.highlight_line("if [ -f file ]; then");
        let kw = spans.iter().find(|s| s.scope == ScopeId::Keyword);
        assert!(kw.is_some());
    }

    // ----- Plain / Markdown -----

    #[test]
    fn plain_returns_no_spans() {
        let h = RegexHighlighter::new(LanguageId::Plain).unwrap();
        let spans = h.highlight_line("some plain text");
        assert!(spans.is_empty());
    }

    #[test]
    fn markdown_returns_no_spans() {
        let h = RegexHighlighter::new(LanguageId::Markdown).unwrap();
        let spans = h.highlight_line("# Heading");
        assert!(spans.is_empty());
    }

    // ----- overlaps helper -----

    #[test]
    fn overlaps_detects_overlap() {
        let existing = vec![HighlightSpan::new(5, 10, ScopeId::Keyword)];
        let new = HighlightSpan::new(8, 12, ScopeId::String);
        assert!(overlaps(&existing, &new));
    }

    #[test]
    fn overlaps_no_overlap_adjacent() {
        let existing = vec![HighlightSpan::new(5, 10, ScopeId::Keyword)];
        let new = HighlightSpan::new(10, 15, ScopeId::String);
        assert!(!overlaps(&existing, &new));
    }

    #[test]
    fn overlaps_no_overlap_before() {
        let existing = vec![HighlightSpan::new(5, 10, ScopeId::Keyword)];
        let new = HighlightSpan::new(0, 5, ScopeId::String);
        assert!(!overlaps(&existing, &new));
    }

    #[test]
    fn overlaps_empty_existing() {
        let existing: Vec<HighlightSpan> = Vec::new();
        let new = HighlightSpan::new(0, 5, ScopeId::String);
        assert!(!overlaps(&existing, &new));
    }

    #[test]
    fn overlaps_contained() {
        let existing = vec![HighlightSpan::new(0, 20, ScopeId::Comment)];
        let new = HighlightSpan::new(5, 10, ScopeId::Keyword);
        assert!(overlaps(&existing, &new));
    }

    // ----- span ordering -----

    #[test]
    fn spans_sorted_by_start() {
        let h = RegexHighlighter::new(LanguageId::Rust).unwrap();
        let spans = h.highlight_line("fn main() { let x = 42; }");
        for window in spans.windows(2) {
            assert!(window[0].start <= window[1].start);
        }
    }

    // ----- no mutual overlap -----

    #[test]
    fn spans_do_not_overlap() {
        let h = RegexHighlighter::new(LanguageId::Rust).unwrap();
        let spans = h.highlight_line("let s = \"hello\"; // comment");
        for (i, a) in spans.iter().enumerate() {
            for b in spans.iter().skip(i + 1) {
                assert!(
                    a.end <= b.start || b.end <= a.start,
                    "spans overlap: {:?} and {:?}",
                    a,
                    b,
                );
            }
        }
    }

    // ----- all languages construct -----

    #[test]
    fn all_languages_construct_without_error() {
        let langs = [
            LanguageId::Rust,
            LanguageId::Python,
            LanguageId::JavaScript,
            LanguageId::TypeScript,
            LanguageId::C,
            LanguageId::Cpp,
            LanguageId::Go,
            LanguageId::Toml,
            LanguageId::Json,
            LanguageId::Shell,
            LanguageId::Markdown,
            LanguageId::Plain,
        ];
        for lang in &langs {
            let h = RegexHighlighter::new(*lang);
            assert!(h.is_ok(), "failed for {:?}: {:?}", lang, h.err());
        }
    }

    // ----- empty line -----

    #[test]
    fn empty_line_returns_no_spans() {
        let h = RegexHighlighter::new(LanguageId::Rust).unwrap();
        let spans = h.highlight_line("");
        assert!(spans.is_empty());
    }
}
