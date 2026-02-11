//! Parses a single command line into program name and args.
//! Supports quoted strings and key=value tokens (e.g. `sum age=2`).

/// Parses a line like `cat path/file --pretty` or `echo "hello world"` or `sum age=2`
/// into (program, args). Respects double and single quotes; first token is program, rest are args.
pub fn parse_cmd_line(line: &str) -> (String, Vec<String>) {
    let tokens = tokenize(line);
    let (program, args) = tokens
        .split_first()
        .map(|(p, rest)| (p.clone(), rest.to_vec()))
        .unwrap_or_else(|| (String::new(), Vec::new()));
    (program, args)
}

/// Splits on whitespace (space, tab), respecting double and single quotes.
/// Quoted substring becomes one token without the quotes.
fn tokenize(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quote: Option<char> = None;

    while let Some(c) = chars.next() {
        match (in_quote, c) {
            (None, '"') | (None, '\'') => {
                in_quote = Some(c);
            }
            (Some(_q), c) if Some(c) == in_quote => {
                in_quote = None;
            }
            (Some(_), _) => {
                cur.push(c);
            }
            (None, c) if c.is_whitespace() => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            (None, _) => {
                cur.push(c);
            }
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let (prog, args) = parse_cmd_line("cat path/file --pretty");
        assert_eq!(prog, "cat");
        assert_eq!(args, &["path/file", "--pretty"]);
    }

    #[test]
    fn test_parse_key_value() {
        let (prog, args) = parse_cmd_line("sum age=2");
        assert_eq!(prog, "sum");
        assert_eq!(args, &["age=2"]);
    }

    #[test]
    fn test_parse_quoted() {
        let (prog, args) = parse_cmd_line(r#"echo "hello world""#);
        assert_eq!(prog, "echo");
        assert_eq!(args, &["hello world"]);
    }

    #[test]
    fn test_parse_empty() {
        let (prog, args) = parse_cmd_line("");
        assert_eq!(prog, "");
        assert!(args.is_empty());
    }

    #[test]
    fn test_parse_program_only() {
        let (prog, args) = parse_cmd_line("ls");
        assert_eq!(prog, "ls");
        assert!(args.is_empty());
    }

    #[test]
    fn test_parse_single_quotes() {
        let (prog, args) = parse_cmd_line("echo 'hello'");
        assert_eq!(prog, "echo");
        assert_eq!(args, &["hello"]);
    }

    #[test]
    fn test_parse_multiple_key_value() {
        let (prog, args) = parse_cmd_line("cmd a=1 b=2");
        assert_eq!(prog, "cmd");
        assert_eq!(args, &["a=1", "b=2"]);
    }
}
