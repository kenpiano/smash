pub mod error;
pub mod highlight;
pub mod language;
pub mod regex_highlighter;
pub mod scope;

pub use error::SyntaxError;
pub use highlight::{HighlightEngine, HighlightSpan};
pub use language::LanguageId;
pub use regex_highlighter::RegexHighlighter;
pub use scope::ScopeId;
