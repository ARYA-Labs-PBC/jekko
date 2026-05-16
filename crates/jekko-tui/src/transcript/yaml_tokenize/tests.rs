//! End-to-end tests for [`super::tokenize_yaml`].

use super::*;

fn scopes(text: &str) -> Vec<YamlScope> {
    tokenize_yaml(text).iter().map(|t| t.scope).collect()
}

#[test]
fn empty_input_yields_no_tokens() {
    assert!(tokenize_yaml("").is_empty());
}

#[test]
fn keys_and_values_are_classified() {
    let tokens = tokenize_yaml("name: jekko\n");
    let kinds = tokens.iter().map(|t| t.scope).collect::<Vec<_>>();
    assert!(kinds.contains(&YamlScope::Property));
    assert!(kinds.contains(&YamlScope::Punctuation));
}

#[test]
fn quoted_string_is_string_scope() {
    let tokens = tokenize_yaml("title: \"hello\"\n");
    assert!(tokens.iter().any(|t| t.scope == YamlScope::StringLit));
}

#[test]
fn boolean_literal_is_tagged() {
    let tokens = tokenize_yaml("enabled: true\n");
    assert!(tokens.iter().any(|t| t.scope == YamlScope::Boolean));
}

#[test]
fn number_literal_is_tagged() {
    let tokens = tokenize_yaml("count: 42\n");
    assert!(tokens.iter().any(|t| t.scope == YamlScope::Number));
}

#[test]
fn negative_number_with_suffix() {
    let tokens = tokenize_yaml("offset: -3.5s\n");
    assert!(tokens.iter().any(|t| t.scope == YamlScope::Number));
}

#[test]
fn comment_runs_to_end_of_line() {
    let tokens = tokenize_yaml("name: x # comment\n");
    let comment = tokens
        .iter()
        .find(|t| t.scope == YamlScope::Comment)
        .unwrap();
    assert!(comment.end > comment.start);
}

#[test]
fn hash_inside_string_is_not_comment() {
    let tokens = tokenize_yaml("key: \"foo # bar\"\n");
    assert!(!tokens.iter().any(|t| t.scope == YamlScope::Comment));
}

#[test]
fn sequence_marker_recognized() {
    let tokens = tokenize_yaml("- one\n- two\n");
    let seqs = tokens
        .iter()
        .filter(|t| t.scope == YamlScope::Sequence)
        .count();
    assert_eq!(seqs, 2);
}

#[test]
fn block_scalar_marker_recognized() {
    let tokens = tokenize_yaml("blob: |\n  hello\n  world\n");
    // The `|` is an operator; the subsequent indented lines tokenize as
    // block content rather than property keys.
    assert!(tokens.iter().any(|t| t.scope == YamlScope::Operator));
    let block_count = tokens
        .iter()
        .filter(|t| t.scope == YamlScope::Block)
        .count();
    assert!(block_count >= 1);
}

#[test]
fn sentinel_lines_are_tagged() {
    let tokens = tokenize_yaml("<<<ZYAL paste>>>\nfoo: 1\n<<<END_ZYAL paste>>>\n");
    let sents = tokens
        .iter()
        .filter(|t| t.scope == YamlScope::Sentinel)
        .count();
    assert_eq!(sents, 2);
}

#[test]
fn zyal_arm_is_sentinel() {
    let tokens = tokenize_yaml("ZYAL_ARM payload\n");
    assert!(tokens.iter().any(|t| t.scope == YamlScope::Sentinel));
}

#[test]
fn tokens_are_in_order() {
    let tokens = tokenize_yaml("a: 1\nb: 2\n");
    let mut last = 0;
    for token in tokens {
        assert!(token.start >= last);
        last = token.start;
    }
}

#[test]
fn does_not_panic_on_partial_input() {
    let _ = tokenize_yaml("name: \"unterminated");
    let _ = tokenize_yaml("[brackets without close");
    let _ = tokenize_yaml("foo: |");
}

#[test]
fn scopes_helper_works() {
    let scopes = scopes("flag: true\n");
    assert!(scopes.contains(&YamlScope::Property));
    assert!(scopes.contains(&YamlScope::Boolean));
}
