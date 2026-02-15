/// Semantic scopes for syntax highlighting.
///
/// Each variant represents a semantic category that can be
/// mapped to a visual style by a theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScopeId {
    Keyword,
    Type,
    Function,
    String,
    Number,
    Comment,
    Operator,
    Punctuation,
    Variable,
    Constant,
    Attribute,
    Macro,
    Namespace,
    Label,
    Plain,
}

impl ScopeId {
    /// Return a readable name for this scope
    /// (used in theme mapping).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Keyword => "keyword",
            Self::Type => "type",
            Self::Function => "function",
            Self::String => "string",
            Self::Number => "number",
            Self::Comment => "comment",
            Self::Operator => "operator",
            Self::Punctuation => "punctuation",
            Self::Variable => "variable",
            Self::Constant => "constant",
            Self::Attribute => "attribute",
            Self::Macro => "macro",
            Self::Namespace => "namespace",
            Self::Label => "label",
            Self::Plain => "plain",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_keyword_as_str() {
        assert_eq!(ScopeId::Keyword.as_str(), "keyword");
    }

    #[test]
    fn scope_type_as_str() {
        assert_eq!(ScopeId::Type.as_str(), "type");
    }

    #[test]
    fn scope_function_as_str() {
        assert_eq!(ScopeId::Function.as_str(), "function");
    }

    #[test]
    fn scope_string_as_str() {
        assert_eq!(ScopeId::String.as_str(), "string");
    }

    #[test]
    fn scope_number_as_str() {
        assert_eq!(ScopeId::Number.as_str(), "number");
    }

    #[test]
    fn scope_comment_as_str() {
        assert_eq!(ScopeId::Comment.as_str(), "comment");
    }

    #[test]
    fn scope_operator_as_str() {
        assert_eq!(ScopeId::Operator.as_str(), "operator");
    }

    #[test]
    fn scope_punctuation_as_str() {
        assert_eq!(ScopeId::Punctuation.as_str(), "punctuation");
    }

    #[test]
    fn scope_variable_as_str() {
        assert_eq!(ScopeId::Variable.as_str(), "variable");
    }

    #[test]
    fn scope_constant_as_str() {
        assert_eq!(ScopeId::Constant.as_str(), "constant");
    }

    #[test]
    fn scope_attribute_as_str() {
        assert_eq!(ScopeId::Attribute.as_str(), "attribute");
    }

    #[test]
    fn scope_macro_as_str() {
        assert_eq!(ScopeId::Macro.as_str(), "macro");
    }

    #[test]
    fn scope_namespace_as_str() {
        assert_eq!(ScopeId::Namespace.as_str(), "namespace");
    }

    #[test]
    fn scope_label_as_str() {
        assert_eq!(ScopeId::Label.as_str(), "label");
    }

    #[test]
    fn scope_plain_as_str() {
        assert_eq!(ScopeId::Plain.as_str(), "plain");
    }

    #[test]
    fn scope_clone_and_eq() {
        let a = ScopeId::Keyword;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn scope_debug() {
        let debug = format!("{:?}", ScopeId::Comment);
        assert_eq!(debug, "Comment");
    }

    #[test]
    fn scope_hash_usable_in_collections() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ScopeId::Keyword);
        set.insert(ScopeId::Keyword);
        assert_eq!(set.len(), 1);
    }
}
