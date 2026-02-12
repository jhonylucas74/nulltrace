//! Pure path resolution (string-only, no DB). Used by the shell for cwd and cd.

/// Resolves a path relative to a base.
///
/// - If `rel` is empty, returns `base` unchanged (or `"/"` if base is empty).
/// - If `rel` starts with `/`, treats it as absolute and normalizes it.
/// - Otherwise joins `base` and `rel`, then normalizes (resolves `.` and `..`).
///
/// # Examples
///
/// - `resolve_relative("/root", "")` → `"/root"`
/// - `resolve_relative("/root", "..")` → `"/"`
/// - `resolve_relative("/root", "a/b")` → `"/root/a/b"`
/// - `resolve_relative("/root", "/tmp")` → `"/tmp"`
/// - `resolve_relative("/a/b", "../c")` → `"/a/c"`
pub fn resolve_relative(base: &str, rel: &str) -> String {
    if rel.is_empty() {
        if base.is_empty() {
            return "/".to_string();
        }
        return base.to_string();
    }
    let to_normalize = if rel.starts_with('/') {
        rel.to_string()
    } else {
        let b = if base.is_empty() || base == "/" {
            "/"
        } else {
            base.trim_end_matches('/')
        };
        format!("{}/{}", b, rel)
    };
    normalize(&to_normalize)
}

/// Normalizes an absolute path: resolve `.` and `..` segments.
fn normalize(path: &str) -> String {
    let segments: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    let mut out: Vec<&str> = Vec::new();
    for seg in segments {
        match seg {
            "." => {}
            ".." => {
                out.pop();
            }
            _ => out.push(seg),
        }
    }
    if out.is_empty() {
        "/".to_string()
    } else {
        "/".to_string() + &out.join("/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_empty_rel_returns_base() {
        assert_eq!(resolve_relative("/root", ""), "/root");
        assert_eq!(resolve_relative("/", ""), "/");
    }

    #[test]
    fn resolve_empty_base_empty_rel_returns_root() {
        assert_eq!(resolve_relative("", ""), "/");
    }

    #[test]
    fn resolve_dot_dot_goes_up() {
        assert_eq!(resolve_relative("/root", ".."), "/");
        assert_eq!(resolve_relative("/a/b", ".."), "/a");
        assert_eq!(resolve_relative("/a/b", "../c"), "/a/c");
    }

    #[test]
    fn resolve_relative_joins() {
        assert_eq!(resolve_relative("/root", "a/b"), "/root/a/b");
        assert_eq!(resolve_relative("/", "tmp"), "/tmp");
    }

    #[test]
    fn resolve_absolute_uses_rel() {
        assert_eq!(resolve_relative("/root", "/tmp"), "/tmp");
        assert_eq!(resolve_relative("/root", "/tmp/../var"), "/var");
    }

    #[test]
    fn normalize_dot_dot() {
        assert_eq!(normalize("/tmp/../var"), "/var");
        assert_eq!(normalize("/a/b/../c"), "/a/c");
        assert_eq!(normalize("/.."), "/");
    }

    #[test]
    fn normalize_dot_skipped() {
        assert_eq!(normalize("/./root/.///a"), "/root/a");
    }
}
