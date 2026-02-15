//! Fuzzy file finder for workspace navigation.
//!
//! Provides fuzzy matching of file paths within a workspace directory,
//! scoring results by match quality for responsive file opening.

use std::path::{Path, PathBuf};

/// A match result from the fuzzy finder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMatch {
    /// The full path to the matched file.
    path: PathBuf,
    /// The relative path from the workspace root.
    relative_path: String,
    /// Match score (higher is better).
    score: i64,
}

impl FileMatch {
    /// Create a new file match.
    pub fn new(path: PathBuf, relative_path: String, score: i64) -> Self {
        Self {
            path,
            relative_path,
            score,
        }
    }

    /// Get the full path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the relative path string.
    pub fn relative_path(&self) -> &str {
        &self.relative_path
    }

    /// Get the match score.
    pub fn score(&self) -> i64 {
        self.score
    }
}

impl PartialOrd for FileMatch {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FileMatch {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher score first, then alphabetical by relative path
        other
            .score
            .cmp(&self.score)
            .then_with(|| self.relative_path.cmp(&other.relative_path))
    }
}

/// Compute a fuzzy match score between a query and a target string.
///
/// Returns `None` if the query doesn't match.
/// Returns `Some(score)` where higher scores indicate better matches.
///
/// Scoring rules:
/// - Each matched character gets base points.
/// - Consecutive matches get a bonus.
/// - Matches at word boundaries (after `/`, `.`, `_`, `-`) get a bonus.
/// - Matches at the start of the string get a bonus.
/// - Shorter targets are preferred (less noise).
pub fn fuzzy_score(query: &str, target: &str) -> Option<i64> {
    if query.is_empty() {
        return Some(0);
    }

    let query_lower: Vec<char> = query.to_lowercase().chars().collect();
    let target_lower: Vec<char> = target.to_lowercase().chars().collect();
    let target_chars: Vec<char> = target.chars().collect();

    if query_lower.len() > target_lower.len() {
        return None;
    }

    // Check if all query chars exist in order
    let mut qi = 0;
    let mut positions = Vec::with_capacity(query_lower.len());

    for (ti, &tc) in target_lower.iter().enumerate() {
        if qi < query_lower.len() && tc == query_lower[qi] {
            positions.push(ti);
            qi += 1;
        }
    }

    if qi < query_lower.len() {
        return None;
    }

    // Score the match
    let mut score: i64 = 0;
    let base_points = 10;

    for (i, &pos) in positions.iter().enumerate() {
        score += base_points;

        // Consecutive match bonus
        if i > 0 && pos == positions[i - 1] + 1 {
            score += 5;
        }

        // Word boundary bonus
        if pos == 0 {
            score += 10;
        } else {
            let prev = target_chars[pos - 1];
            if prev == '/' || prev == '\\' || prev == '.' || prev == '_' || prev == '-' {
                score += 8;
            }
            // Camel case boundary
            if target_chars[pos].is_uppercase() && prev.is_lowercase() {
                score += 5;
            }
        }

        // Exact case match bonus
        if target_chars[pos] == query.chars().nth(i).unwrap_or(' ') {
            score += 2;
        }
    }

    // Prefer shorter paths
    let length_penalty = (target_lower.len() as i64).saturating_sub(query_lower.len() as i64);
    score -= length_penalty;

    // Bonus for matching at the end (filename portion)
    if let Some(last_sep) = target.rfind('/') {
        let filename_start = last_sep + 1;
        if !positions.is_empty() && positions[0] >= filename_start {
            score += 15; // All matches in filename
        }
    }

    Some(score)
}

/// Default directories and file patterns to ignore.
const DEFAULT_IGNORE_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "node_modules",
    "target",
    "__pycache__",
    ".tox",
    ".mypy_cache",
    ".pytest_cache",
    "dist",
    "build",
    ".next",
    ".nuxt",
    "vendor",
    ".venv",
    "venv",
];

/// Check if a directory name should be ignored.
fn is_ignored_dir(name: &str) -> bool {
    DEFAULT_IGNORE_DIRS.contains(&name)
}

/// Walk a directory tree and collect all file paths.
///
/// Respects common ignore patterns (e.g., .git, node_modules, target).
/// Returns paths relative to the root directory.
pub fn walk_directory(root: &Path, max_files: usize) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        if files.len() >= max_files {
            break;
        }

        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            if files.len() >= max_files {
                break;
            }

            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Skip hidden files/dirs (starting with .)
            if name_str.starts_with('.') && name_str != "." {
                continue;
            }

            if path.is_dir() {
                if !is_ignored_dir(&name_str) {
                    stack.push(path);
                }
            } else if path.is_file() {
                if let Ok(rel) = path.strip_prefix(root) {
                    files.push(rel.to_path_buf());
                }
            }
        }
    }

    files.sort();
    files
}

/// The fuzzy file finder.
///
/// Indexes files in a workspace directory and provides fuzzy matching.
#[derive(Debug, Clone)]
pub struct FileFinder {
    /// Root directory of the workspace.
    root: PathBuf,
    /// Cached list of relative file paths.
    files: Vec<PathBuf>,
    /// Maximum number of files to index.
    max_files: usize,
}

impl FileFinder {
    /// Create a new file finder rooted at the given directory.
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            files: Vec::new(),
            max_files: 100_000,
        }
    }

    /// Create a file finder with a custom file limit.
    pub fn with_max_files(root: PathBuf, max_files: usize) -> Self {
        Self {
            root,
            files: Vec::new(),
            max_files,
        }
    }

    /// Index the workspace directory.
    pub fn index(&mut self) {
        self.files = walk_directory(&self.root, self.max_files);
    }

    /// Get the number of indexed files.
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Get the root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Search for files matching the query.
    ///
    /// Returns results sorted by match score (best first), limited to `max_results`.
    pub fn search(&self, query: &str, max_results: usize) -> Vec<FileMatch> {
        if query.is_empty() {
            // Return all files (up to limit) when query is empty
            return self
                .files
                .iter()
                .take(max_results)
                .map(|p| {
                    let rel = p.to_string_lossy().to_string();
                    FileMatch::new(self.root.join(p), rel, 0)
                })
                .collect();
        }

        let mut matches: Vec<FileMatch> = self
            .files
            .iter()
            .filter_map(|path| {
                let path_str = path.to_string_lossy();
                fuzzy_score(query, &path_str)
                    .map(|score| FileMatch::new(self.root.join(path), path_str.to_string(), score))
            })
            .collect();

        matches.sort();
        matches.truncate(max_results);
        matches
    }

    /// Add a file path to the index without re-walking the directory.
    pub fn add_file(&mut self, relative_path: PathBuf) {
        if !self.files.contains(&relative_path) {
            self.files.push(relative_path);
            self.files.sort();
        }
    }

    /// Remove a file path from the index.
    pub fn remove_file(&mut self, relative_path: &Path) {
        self.files.retain(|p| p != relative_path);
    }

    /// Clear the file index.
    pub fn clear(&mut self) {
        self.files.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- fuzzy_score tests ---

    #[test]
    fn fuzzy_score_empty_query_matches_anything() {
        assert_eq!(fuzzy_score("", "anything"), Some(0));
    }

    #[test]
    fn fuzzy_score_exact_match() {
        let score = fuzzy_score("main.rs", "main.rs").unwrap();
        assert!(score > 0);
    }

    #[test]
    fn fuzzy_score_no_match() {
        assert!(fuzzy_score("xyz", "abc").is_none());
    }

    #[test]
    fn fuzzy_score_partial_match() {
        let score = fuzzy_score("mn", "main.rs").unwrap();
        assert!(score > 0);
    }

    #[test]
    fn fuzzy_score_case_insensitive() {
        let score1 = fuzzy_score("main", "Main.rs").unwrap();
        let score2 = fuzzy_score("MAIN", "main.rs").unwrap();
        // Both should match, scores may differ slightly due to exact case bonus
        assert!(score1 > 0);
        assert!(score2 > 0);
    }

    #[test]
    fn fuzzy_score_query_longer_than_target() {
        assert!(fuzzy_score("very_long_query", "short").is_none());
    }

    #[test]
    fn fuzzy_score_consecutive_bonus() {
        let consecutive = fuzzy_score("abc", "abcdef").unwrap();
        let spread = fuzzy_score("abc", "axbxcx").unwrap();
        assert!(consecutive > spread, "consecutive should score higher");
    }

    #[test]
    fn fuzzy_score_word_boundary_bonus() {
        let boundary = fuzzy_score("mr", "main.rs").unwrap();
        let middle = fuzzy_score("mr", "xmxrxx").unwrap();
        assert!(boundary > middle, "word boundary match should score higher");
    }

    #[test]
    fn fuzzy_score_filename_match_bonus() {
        let filename = fuzzy_score("main", "src/main.rs").unwrap();
        assert!(filename > 0);
    }

    #[test]
    fn fuzzy_score_prefers_shorter_paths() {
        let short = fuzzy_score("main", "main.rs").unwrap();
        let long = fuzzy_score("main", "very/long/path/to/main.rs").unwrap();
        assert!(short > long, "shorter path should score higher");
    }

    #[test]
    fn fuzzy_score_path_separators() {
        let score = fuzzy_score("sl", "src/lib.rs").unwrap();
        assert!(score > 0);
    }

    // --- FileMatch tests ---

    #[test]
    fn file_match_new() {
        let m = FileMatch::new(PathBuf::from("/a/b"), "b".to_string(), 50);
        assert_eq!(m.path(), Path::new("/a/b"));
        assert_eq!(m.relative_path(), "b");
        assert_eq!(m.score(), 50);
    }

    #[test]
    fn file_match_ordering_by_score() {
        let high = FileMatch::new(PathBuf::from("a"), "a".to_string(), 100);
        let low = FileMatch::new(PathBuf::from("b"), "b".to_string(), 50);
        let mut matches = [low.clone(), high.clone()];
        matches.sort();
        assert_eq!(matches[0].score(), 100); // Higher score first
    }

    #[test]
    fn file_match_ordering_same_score() {
        let a = FileMatch::new(PathBuf::from("a"), "a.rs".to_string(), 50);
        let b = FileMatch::new(PathBuf::from("b"), "b.rs".to_string(), 50);
        let mut matches = [b.clone(), a.clone()];
        matches.sort();
        assert_eq!(matches[0].relative_path(), "a.rs"); // Alphabetical
    }

    #[test]
    fn file_match_clone_and_eq() {
        let m = FileMatch::new(PathBuf::from("a"), "a".to_string(), 50);
        let cloned = m.clone();
        assert_eq!(m, cloned);
    }

    // --- walk_directory tests ---

    #[test]
    fn walk_directory_nonexistent() {
        let files = walk_directory(Path::new("/nonexistent/path/xyz"), 100);
        assert!(files.is_empty());
    }

    #[test]
    fn walk_directory_with_limit() {
        let tmp = std::env::temp_dir().join("smash_test_walk_limit");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        for i in 0..10 {
            std::fs::write(tmp.join(format!("file{}.txt", i)), "").unwrap();
        }

        let files = walk_directory(&tmp, 3);
        assert!(files.len() <= 3);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn walk_directory_ignores_hidden() {
        let tmp = std::env::temp_dir().join("smash_test_walk_hidden");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join(".hidden_dir")).unwrap();
        std::fs::write(tmp.join("visible.txt"), "").unwrap();
        std::fs::write(tmp.join(".hidden_file"), "").unwrap();

        let files = walk_directory(&tmp, 100);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], PathBuf::from("visible.txt"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn walk_directory_ignores_target_dir() {
        let tmp = std::env::temp_dir().join("smash_test_walk_target");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("target")).unwrap();
        std::fs::create_dir_all(tmp.join("src")).unwrap();
        std::fs::write(tmp.join("target/debug.rs"), "").unwrap();
        std::fs::write(tmp.join("src/main.rs"), "").unwrap();

        let files = walk_directory(&tmp, 100);
        assert_eq!(files.len(), 1);
        assert!(files[0].to_string_lossy().contains("main.rs"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn walk_directory_ignores_node_modules() {
        let tmp = std::env::temp_dir().join("smash_test_walk_node");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("node_modules/dep")).unwrap();
        std::fs::write(tmp.join("node_modules/dep/index.js"), "").unwrap();
        std::fs::write(tmp.join("index.js"), "").unwrap();

        let files = walk_directory(&tmp, 100);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], PathBuf::from("index.js"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn walk_directory_sorted() {
        let tmp = std::env::temp_dir().join("smash_test_walk_sorted");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("c.txt"), "").unwrap();
        std::fs::write(tmp.join("a.txt"), "").unwrap();
        std::fs::write(tmp.join("b.txt"), "").unwrap();

        let files = walk_directory(&tmp, 100);
        assert_eq!(files[0], PathBuf::from("a.txt"));
        assert_eq!(files[1], PathBuf::from("b.txt"));
        assert_eq!(files[2], PathBuf::from("c.txt"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // --- is_ignored_dir tests ---

    #[test]
    fn ignored_dirs() {
        assert!(is_ignored_dir(".git"));
        assert!(is_ignored_dir("node_modules"));
        assert!(is_ignored_dir("target"));
        assert!(is_ignored_dir("__pycache__"));
        assert!(!is_ignored_dir("src"));
        assert!(!is_ignored_dir("lib"));
    }

    // --- FileFinder tests ---

    #[test]
    fn file_finder_new() {
        let finder = FileFinder::new(PathBuf::from("/workspace"));
        assert_eq!(finder.root(), Path::new("/workspace"));
        assert_eq!(finder.file_count(), 0);
    }

    #[test]
    fn file_finder_with_max_files() {
        let finder = FileFinder::with_max_files(PathBuf::from("/ws"), 500);
        assert_eq!(finder.file_count(), 0);
    }

    #[test]
    fn file_finder_add_and_remove() {
        let mut finder = FileFinder::new(PathBuf::from("/ws"));
        finder.add_file(PathBuf::from("src/main.rs"));
        finder.add_file(PathBuf::from("src/lib.rs"));
        assert_eq!(finder.file_count(), 2);

        // Duplicate add
        finder.add_file(PathBuf::from("src/main.rs"));
        assert_eq!(finder.file_count(), 2);

        finder.remove_file(Path::new("src/main.rs"));
        assert_eq!(finder.file_count(), 1);
    }

    #[test]
    fn file_finder_clear() {
        let mut finder = FileFinder::new(PathBuf::from("/ws"));
        finder.add_file(PathBuf::from("a.rs"));
        finder.add_file(PathBuf::from("b.rs"));
        finder.clear();
        assert_eq!(finder.file_count(), 0);
    }

    #[test]
    fn file_finder_search_empty_query() {
        let mut finder = FileFinder::new(PathBuf::from("/ws"));
        finder.add_file(PathBuf::from("a.rs"));
        finder.add_file(PathBuf::from("b.rs"));

        let results = finder.search("", 10);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn file_finder_search_with_query() {
        let mut finder = FileFinder::new(PathBuf::from("/ws"));
        finder.add_file(PathBuf::from("src/main.rs"));
        finder.add_file(PathBuf::from("src/lib.rs"));
        finder.add_file(PathBuf::from("Cargo.toml"));

        let results = finder.search("main", 10);
        assert_eq!(results.len(), 1);
        assert!(results[0].relative_path().contains("main"));
    }

    #[test]
    fn file_finder_search_fuzzy() {
        let mut finder = FileFinder::new(PathBuf::from("/ws"));
        finder.add_file(PathBuf::from("src/main.rs"));
        finder.add_file(PathBuf::from("src/lib.rs"));

        let results = finder.search("sr", 10);
        // Both files should match "sr" from "src/"
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn file_finder_search_max_results() {
        let mut finder = FileFinder::new(PathBuf::from("/ws"));
        for i in 0..20 {
            finder.add_file(PathBuf::from(format!("file{}.rs", i)));
        }

        let results = finder.search("file", 5);
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn file_finder_search_no_match() {
        let mut finder = FileFinder::new(PathBuf::from("/ws"));
        finder.add_file(PathBuf::from("main.rs"));

        let results = finder.search("xyz", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn file_finder_index_real_dir() {
        let tmp = std::env::temp_dir().join("smash_test_finder_index");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("src")).unwrap();
        std::fs::write(tmp.join("src/main.rs"), "fn main() {}").unwrap();
        std::fs::write(tmp.join("Cargo.toml"), "[package]").unwrap();

        let mut finder = FileFinder::new(tmp.clone());
        finder.index();
        assert_eq!(finder.file_count(), 2);

        let results = finder.search("main", 10);
        assert_eq!(results.len(), 1);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn file_finder_clone() {
        let mut finder = FileFinder::new(PathBuf::from("/ws"));
        finder.add_file(PathBuf::from("a.rs"));
        let cloned = finder.clone();
        assert_eq!(cloned.file_count(), 1);
    }

    #[test]
    fn file_finder_debug() {
        let finder = FileFinder::new(PathBuf::from("/ws"));
        let debug = format!("{:?}", finder);
        assert!(debug.contains("FileFinder"));
    }
}
