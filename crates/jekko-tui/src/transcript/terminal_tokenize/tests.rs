use super::*;

#[test]
fn strip_ansi_removes_csi() {
    let s = "\x1b[31mred\x1b[0m";
    assert_eq!(strip_ansi(s), "red");
}

#[test]
fn strip_ansi_removes_osc_terminator_bel() {
    let s = "\x1b]0;title\x07after";
    assert_eq!(strip_ansi(s), "after");
}

#[test]
fn strip_ansi_removes_osc_terminator_st() {
    let s = "\x1b]0;title\x1b\\after";
    assert_eq!(strip_ansi(s), "after");
}

#[test]
fn strip_ansi_passes_plain_text() {
    let s = "hello world";
    assert_eq!(strip_ansi(s), "hello world");
}

#[test]
fn tokenizes_quoted_string() {
    let tokens = tokenize_terminal(r#"echo "hi""#);
    let strings: Vec<_> = tokens
        .iter()
        .filter(|t| t.scope == TerminalScope::StringLit)
        .collect();
    assert_eq!(strings.len(), 1);
    assert_eq!(strings[0].end - strings[0].start, 4);
}

#[test]
fn tokenizes_shell_prompt_and_command() {
    let tokens = tokenize_terminal("$ ls -la");
    assert!(tokens.iter().any(|t| t.scope == TerminalScope::Prompt));
    let cmd: Vec<_> = tokens
        .iter()
        .filter(|t| t.scope == TerminalScope::Command)
        .collect();
    assert_eq!(cmd.len(), 1);
}

#[test]
fn tokenizes_pass_fail_badges() {
    let tokens = tokenize_terminal("PASS x FAIL y");
    let kinds: Vec<TerminalScope> = tokens.iter().map(|t| t.scope).collect();
    assert!(kinds.contains(&TerminalScope::Success));
    assert!(kinds.contains(&TerminalScope::Error));
}

#[test]
fn tokenizes_warning_badge() {
    let tokens = tokenize_terminal("WARN deprecation");
    assert!(tokens.iter().any(|t| t.scope == TerminalScope::Warning));
}

#[test]
fn tokenizes_number_and_time() {
    let tokens = tokenize_terminal("ran in 12ms");
    let scopes: Vec<TerminalScope> = tokens.iter().map(|t| t.scope).collect();
    assert!(scopes.contains(&TerminalScope::Time));
}

#[test]
fn tokenizes_bracketed_time() {
    let tokens = tokenize_terminal("[ 0.012s] ok");
    assert!(tokens.iter().any(|t| t.scope == TerminalScope::Time));
}

#[test]
fn tokenizes_keywords() {
    let tokens = tokenize_terminal("value=true other=false");
    assert!(
        tokens
            .iter()
            .filter(|t| t.scope == TerminalScope::Keyword)
            .count()
            >= 1
    );
}

#[test]
fn punctuation_is_tagged() {
    let tokens = tokenize_terminal("[a]{b}(c)");
    let punct = tokens
        .iter()
        .filter(|t| t.scope == TerminalScope::Punctuation)
        .count();
    assert!(punct >= 6);
}

#[test]
fn tokens_do_not_overlap() {
    let tokens = tokenize_terminal("\"123\" 456");
    let mut cursor = 0;
    for token in tokens {
        assert!(token.start >= cursor);
        cursor = token.end;
    }
}

#[test]
fn check_glyphs_are_handled() {
    let tokens = tokenize_terminal("\u{2713} ok");
    assert!(tokens.iter().any(|t| t.scope == TerminalScope::Success));
}

#[test]
fn cross_glyph_x_is_error() {
    let tokens = tokenize_terminal("\u{2717} bad");
    assert!(tokens.iter().any(|t| t.scope == TerminalScope::Error));
}
