//! Rust utilities for find/grep: search_files (path walk with filters) and search_files_content (grep-like).
//! Used by bin find and bin grep via Lua fs.search_files / fs.search_files_content.

use crate::db::fs_service::{FsService, FsStat};
use chrono::Utc;
use regex::Regex;
use std::path::Path;
use uuid::Uuid;

/// Options for search_files (find-like).
#[derive(Debug, Default, Clone)]
pub struct SearchFilesOptions {
    /// Glob pattern for basename (e.g. "*.lua"). Matched case-sensitive.
    pub name: Option<String>,
    /// Glob pattern for basename, case-insensitive.
    pub iname: Option<String>,
    /// "file" or "directory".
    pub type_filter: Option<String>,
    /// Size spec: "+n", "-n", "n", with optional K/M/G suffix.
    pub size_spec: Option<String>,
    /// Owner username.
    pub user: Option<String>,
    /// Mtime in days: positive = modified >= n days ago, negative = modified <= n days ago.
    pub mtime_days: Option<i64>,
}

/// One matching line from search_files_content.
#[derive(Debug, Clone)]
pub struct ContentMatch {
    pub path: String,
    pub line_num: u32,
    pub line: String,
}

/// Options for search_files_content (grep-like).
#[derive(Debug, Default, Clone)]
pub struct SearchContentOptions {
    /// If true, pattern is a regex; otherwise literal substring.
    pub regex: bool,
    pub case_insensitive: bool,
}

fn basename(path: &str) -> &str {
    Path::new(path)
        .file_name()
        .and_then(|p| p.to_str())
        .unwrap_or(path)
}

/// Convert find-style glob to regex pattern (anchored). * -> .*, ? -> ., escape other regex metachars.
fn glob_to_regex(glob: &str) -> String {
    let mut out = String::with_capacity(glob.len() * 2);
    out.push('^');
    for c in glob.chars() {
        match c {
            '*' => out.push_str(".*"),
            '?' => out.push('.'),
            '.' | '+' | '-' | '^' | '$' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out.push('$');
    out
}

fn name_matches(basename_str: &str, glob_pattern: &str, case_insensitive: bool) -> bool {
    let (b, pat) = if case_insensitive {
        (
            basename_str.to_lowercase(),
            glob_pattern.to_lowercase(),
        )
    } else {
        (basename_str.to_string(), glob_pattern.to_string())
    };
    let re_str = glob_to_regex(&pat);
    let re = match Regex::new(&re_str) {
        Ok(r) => r,
        Err(_) => return false,
    };
    re.is_match(&b)
}

fn size_matches(size_bytes: i64, spec: &str) -> bool {
    let s = spec.trim();
    let (sign, s) = if s.starts_with('+') {
        (1, s[1..].trim())
    } else if s.starts_with('-') {
        (-1, s[1..].trim())
    } else {
        (0, s)
    };
    let mult = if s.ends_with('K') {
        let n: i64 = s[..s.len() - 1].trim().parse().unwrap_or(0);
        n * 1024
    } else if s.ends_with('M') {
        let n: i64 = s[..s.len() - 1].trim().parse().unwrap_or(0);
        n * 1024 * 1024
    } else if s.ends_with('G') {
        let n: i64 = s[..s.len() - 1].trim().parse().unwrap_or(0);
        n * 1024 * 1024 * 1024
    } else {
        s.parse::<i64>().unwrap_or(0)
    };
    match sign {
        1 => size_bytes > mult,
        -1 => size_bytes < mult,
        _ => size_bytes == mult,
    }
}

fn mtime_matches(mtime_secs: i64, days: Option<i64>, now_secs: i64) -> bool {
    let Some(days) = days else {
        return true;
    };
    if mtime_secs == 0 {
        return false;
    }
    let age_days = (now_secs - mtime_secs) as f64 / 86400.0;
    if days < 0 {
        age_days <= days.abs() as f64
    } else if days > 0 {
        age_days >= days as f64
    } else {
        age_days < 1.0
    }
}

fn stat_matches(st: &FsStat, path: &str, opts: &SearchFilesOptions, now_secs: i64) -> bool {
    let base = basename(path);
    if let Some(ref name) = opts.name {
        if !name_matches(base, name, false) {
            return false;
        }
    }
    if let Some(ref iname) = opts.iname {
        if !name_matches(base, iname, true) {
            return false;
        }
    }
    if let Some(ref tf) = opts.type_filter {
        let t = tf.as_str();
        if t == "f" && st.node_type != "file" {
            return false;
        }
        if t == "d" && st.node_type != "directory" {
            return false;
        }
    }
    if let Some(ref size_spec) = opts.size_spec {
        if !size_matches(st.size_bytes, size_spec) {
            return false;
        }
    }
    if let Some(ref user) = opts.user {
        if st.owner != *user {
            return false;
        }
    }
    if opts.mtime_days.is_some() {
        let mtime = st.updated_at.timestamp();
        if !mtime_matches(mtime, opts.mtime_days, now_secs) {
            return false;
        }
    }
    true
}

/// Recursive walk: collect all paths under root that match opts. Uses iterative stack to avoid async recursion.
pub async fn search_files(
    fs: &FsService,
    vm_id: Uuid,
    root_path: &str,
    opts: &SearchFilesOptions,
) -> Result<Vec<String>, sqlx::Error> {
    let mut results = Vec::new();
    let now_secs = Utc::now().timestamp();
    let mut stack = vec![root_path.to_string()];
    while let Some(root) = stack.pop() {
        let st = match fs.stat_at(vm_id, &root).await? {
            Some(s) => s,
            None => continue,
        };
        if stat_matches(&st, &root, opts, now_secs) {
            results.push(root.clone());
        }
        if st.node_type != "directory" {
            continue;
        }
        let entries = fs.ls(vm_id, &root).await?;
        for entry in entries {
            let full = if root == "/" {
                format!("/{}", entry.name)
            } else {
                format!("{}/{}", root, entry.name)
            };
            stack.push(full);
        }
    }
    Ok(results)
}

/// Expand paths: directories are replaced by all files under them (recursive). Returns only file paths.
async fn expand_to_files(
    fs: &FsService,
    vm_id: Uuid,
    paths: &[String],
) -> Result<Vec<String>, sqlx::Error> {
    let mut files = Vec::new();
    for p in paths {
        let st = match fs.stat_at(vm_id, p).await? {
            Some(s) => s,
            None => continue,
        };
        if st.node_type == "file" {
            files.push(p.clone());
        } else if st.node_type == "directory" {
            let opts = SearchFilesOptions {
                type_filter: Some("file".to_string()),
                ..Default::default()
            };
            let sub = search_files(fs, vm_id, p, &opts).await?;
            files.extend(sub);
        }
    }
    Ok(files)
}

/// Search file contents for pattern; return (path, line_num, line) for each match.
pub async fn search_files_content(
    fs: &FsService,
    vm_id: Uuid,
    paths: &[String],
    pattern: &str,
    opts: &SearchContentOptions,
) -> Result<Vec<ContentMatch>, Box<dyn std::error::Error + Send + Sync>> {
    let files = expand_to_files(fs, vm_id, paths).await?;
    let mut results = Vec::new();

    let re = if opts.regex {
        let re_str = if opts.case_insensitive {
            format!("(?i){}", pattern)
        } else {
            pattern.to_string()
        };
        Regex::new(&re_str).map_err(|e| e.to_string())?
    } else {
        let re_str = if opts.case_insensitive {
            format!("(?i){}", regex::escape(pattern))
        } else {
            regex::escape(pattern)
        };
        Regex::new(&re_str).map_err(|e| e.to_string())?
    };

    for path in files {
        let content = match fs.read_file(vm_id, &path).await? {
            Some((data, _)) => data,
            None => continue,
        };
        let text = String::from_utf8_lossy(&content);
        let text = text.as_ref();
        let mut line_num = 0u32;
        for line in text.split_inclusive('\n') {
            line_num += 1;
            let line_stripped = line.strip_suffix('\n').unwrap_or(line);
            if re.is_match(line_stripped) {
                results.push(ContentMatch {
                    path: path.clone(),
                    line_num,
                    line: line_stripped.to_string(),
                });
            }
        }
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_to_regex() {
        assert_eq!(glob_to_regex("*.lua"), r"^.*\.lua$");
        assert_eq!(glob_to_regex("a?c"), r"^a.c$");
        assert_eq!(glob_to_regex("a.c"), r"^a\.c$");
    }

    #[test]
    fn test_name_matches() {
        assert!(name_matches("foo.lua", "*.lua", false));
        assert!(!name_matches("foo.txt", "*.lua", false));
        assert!(name_matches("FOO.LUA", "*.lua", true));
    }

    #[test]
    fn test_size_matches() {
        assert!(size_matches(100, "100"));
        assert!(size_matches(200, "+100"));
        assert!(size_matches(50, "-100"));
        assert!(size_matches(1024, "1K"));
        assert!(size_matches(2048, "+1K"));
    }

    #[test]
    fn test_mtime_matches() {
        let now = 1000000i64;
        // modified 2 days ago -> age_days = 2
        assert!(mtime_matches(now - 172800, Some(2), now));
        assert!(!mtime_matches(now - 172800, Some(3), now));
    }

    #[test]
    fn test_basename() {
        assert_eq!(basename("/tmp/foo.txt"), "foo.txt");
        assert_eq!(basename("/"), "/");
    }

    #[test]
    fn test_glob_to_regex_escapes_special() {
        assert!(name_matches("a.c", "a.c", false));
        assert!(name_matches("a+c", "a+c", false));
    }

    #[test]
    fn test_size_matches_m_and_g() {
        assert!(size_matches(1024 * 1024, "1M"));
        assert!(size_matches(1024 * 1024 * 1024, "1G"));
        assert!(size_matches(2048 * 1024 * 1024, "+1G"));
    }
}
