use std::path::Path;

/// Known languages for Phase 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageId {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    C,
    Cpp,
    Go,
    Toml,
    Json,
    Markdown,
    Shell,
    Plain,
}

impl LanguageId {
    /// Detect language from a file extension (without the dot).
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Self::Rust,
            "py" | "pyw" => Self::Python,
            "js" | "mjs" | "cjs" => Self::JavaScript,
            "ts" | "tsx" => Self::TypeScript,
            "c" | "h" => Self::C,
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Self::Cpp,
            "go" => Self::Go,
            "toml" => Self::Toml,
            "json" => Self::Json,
            "md" | "markdown" => Self::Markdown,
            "sh" | "bash" | "zsh" => Self::Shell,
            _ => Self::Plain,
        }
    }

    /// Detect language from a file path.
    ///
    /// Checks special filenames first (e.g. `Makefile`,
    /// `Cargo.toml`), then falls back to extension-based
    /// detection.
    pub fn from_path(path: &Path) -> Self {
        // Check special filenames
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            match name {
                "Makefile" | "makefile" | "GNUmakefile" => {
                    return Self::Shell;
                }
                "Cargo.toml" | "Cargo.lock" => {
                    return Self::Toml;
                }
                "Dockerfile" => return Self::Shell,
                _ => {}
            }
        }
        // Fall back to extension
        path.extension()
            .and_then(|e| e.to_str())
            .map(Self::from_extension)
            .unwrap_or(Self::Plain)
    }

    /// Return the canonical name of this language.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::Go => "go",
            Self::Toml => "toml",
            Self::Json => "json",
            Self::Markdown => "markdown",
            Self::Shell => "shell",
            Self::Plain => "plain",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ----- from_extension tests -----

    #[test]
    fn ext_rs_is_rust() {
        assert_eq!(LanguageId::from_extension("rs"), LanguageId::Rust);
    }

    #[test]
    fn ext_py_is_python() {
        assert_eq!(LanguageId::from_extension("py"), LanguageId::Python);
    }

    #[test]
    fn ext_pyw_is_python() {
        assert_eq!(LanguageId::from_extension("pyw"), LanguageId::Python);
    }

    #[test]
    fn ext_js_is_javascript() {
        assert_eq!(LanguageId::from_extension("js"), LanguageId::JavaScript);
    }

    #[test]
    fn ext_mjs_is_javascript() {
        assert_eq!(LanguageId::from_extension("mjs"), LanguageId::JavaScript);
    }

    #[test]
    fn ext_cjs_is_javascript() {
        assert_eq!(LanguageId::from_extension("cjs"), LanguageId::JavaScript);
    }

    #[test]
    fn ext_ts_is_typescript() {
        assert_eq!(LanguageId::from_extension("ts"), LanguageId::TypeScript);
    }

    #[test]
    fn ext_tsx_is_typescript() {
        assert_eq!(LanguageId::from_extension("tsx"), LanguageId::TypeScript);
    }

    #[test]
    fn ext_c_is_c() {
        assert_eq!(LanguageId::from_extension("c"), LanguageId::C);
    }

    #[test]
    fn ext_h_is_c() {
        assert_eq!(LanguageId::from_extension("h"), LanguageId::C);
    }

    #[test]
    fn ext_cpp_is_cpp() {
        assert_eq!(LanguageId::from_extension("cpp"), LanguageId::Cpp);
    }

    #[test]
    fn ext_cc_is_cpp() {
        assert_eq!(LanguageId::from_extension("cc"), LanguageId::Cpp);
    }

    #[test]
    fn ext_cxx_is_cpp() {
        assert_eq!(LanguageId::from_extension("cxx"), LanguageId::Cpp);
    }

    #[test]
    fn ext_hpp_is_cpp() {
        assert_eq!(LanguageId::from_extension("hpp"), LanguageId::Cpp);
    }

    #[test]
    fn ext_hxx_is_cpp() {
        assert_eq!(LanguageId::from_extension("hxx"), LanguageId::Cpp);
    }

    #[test]
    fn ext_go_is_go() {
        assert_eq!(LanguageId::from_extension("go"), LanguageId::Go);
    }

    #[test]
    fn ext_toml_is_toml() {
        assert_eq!(LanguageId::from_extension("toml"), LanguageId::Toml);
    }

    #[test]
    fn ext_json_is_json() {
        assert_eq!(LanguageId::from_extension("json"), LanguageId::Json);
    }

    #[test]
    fn ext_md_is_markdown() {
        assert_eq!(LanguageId::from_extension("md"), LanguageId::Markdown);
    }

    #[test]
    fn ext_markdown_is_markdown() {
        assert_eq!(LanguageId::from_extension("markdown"), LanguageId::Markdown);
    }

    #[test]
    fn ext_sh_is_shell() {
        assert_eq!(LanguageId::from_extension("sh"), LanguageId::Shell);
    }

    #[test]
    fn ext_bash_is_shell() {
        assert_eq!(LanguageId::from_extension("bash"), LanguageId::Shell);
    }

    #[test]
    fn ext_zsh_is_shell() {
        assert_eq!(LanguageId::from_extension("zsh"), LanguageId::Shell);
    }

    #[test]
    fn ext_unknown_is_plain() {
        assert_eq!(LanguageId::from_extension("xyz"), LanguageId::Plain);
    }

    #[test]
    fn ext_case_insensitive() {
        assert_eq!(LanguageId::from_extension("RS"), LanguageId::Rust);
        assert_eq!(LanguageId::from_extension("Py"), LanguageId::Python);
    }

    // ----- from_path tests -----

    #[test]
    fn path_makefile_is_shell() {
        let p = PathBuf::from("src/Makefile");
        assert_eq!(LanguageId::from_path(&p), LanguageId::Shell);
    }

    #[test]
    fn path_lowercase_makefile_is_shell() {
        let p = PathBuf::from("makefile");
        assert_eq!(LanguageId::from_path(&p), LanguageId::Shell);
    }

    #[test]
    fn path_gnumakefile_is_shell() {
        let p = PathBuf::from("GNUmakefile");
        assert_eq!(LanguageId::from_path(&p), LanguageId::Shell);
    }

    #[test]
    fn path_cargo_toml_is_toml() {
        let p = PathBuf::from("Cargo.toml");
        assert_eq!(LanguageId::from_path(&p), LanguageId::Toml);
    }

    #[test]
    fn path_cargo_lock_is_toml() {
        let p = PathBuf::from("Cargo.lock");
        assert_eq!(LanguageId::from_path(&p), LanguageId::Toml);
    }

    #[test]
    fn path_dockerfile_is_shell() {
        let p = PathBuf::from("Dockerfile");
        assert_eq!(LanguageId::from_path(&p), LanguageId::Shell);
    }

    #[test]
    fn path_regular_rust_file() {
        let p = PathBuf::from("src/main.rs");
        assert_eq!(LanguageId::from_path(&p), LanguageId::Rust);
    }

    #[test]
    fn path_no_extension_is_plain() {
        let p = PathBuf::from("README");
        assert_eq!(LanguageId::from_path(&p), LanguageId::Plain);
    }

    #[test]
    fn path_nested_python() {
        let p = PathBuf::from("a/b/c/script.py");
        assert_eq!(LanguageId::from_path(&p), LanguageId::Python);
    }

    // ----- as_str tests -----

    #[test]
    fn as_str_all_variants() {
        assert_eq!(LanguageId::Rust.as_str(), "rust");
        assert_eq!(LanguageId::Python.as_str(), "python");
        assert_eq!(LanguageId::JavaScript.as_str(), "javascript");
        assert_eq!(LanguageId::TypeScript.as_str(), "typescript");
        assert_eq!(LanguageId::C.as_str(), "c");
        assert_eq!(LanguageId::Cpp.as_str(), "cpp");
        assert_eq!(LanguageId::Go.as_str(), "go");
        assert_eq!(LanguageId::Toml.as_str(), "toml");
        assert_eq!(LanguageId::Json.as_str(), "json");
        assert_eq!(LanguageId::Markdown.as_str(), "markdown");
        assert_eq!(LanguageId::Shell.as_str(), "shell");
        assert_eq!(LanguageId::Plain.as_str(), "plain");
    }

    #[test]
    fn language_clone_and_eq() {
        let a = LanguageId::Rust;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn language_debug() {
        let debug = format!("{:?}", LanguageId::Python);
        assert_eq!(debug, "Python");
    }

    #[test]
    fn language_hash_usable_in_collections() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(LanguageId::Rust);
        set.insert(LanguageId::Rust);
        assert_eq!(set.len(), 1);
    }
}
