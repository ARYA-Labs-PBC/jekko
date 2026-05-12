//! Layer 3 — permission matcher property + table tests.

use proptest::prelude::*;
use sandboxctl::permission::{Decision, Matcher};

fn argv(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

#[test]
fn whitelist_default_deny() {
    let m = Matcher::new(&vec!["just *".into()], &vec![]).unwrap();
    assert_eq!(
        m.evaluate(&argv(&["echo", "hi"])),
        Decision::NoAllowMatched
    );
}

#[test]
fn deny_wins_intersection() {
    let m = Matcher::new(
        &vec!["cargo *".into()],
        &vec!["cargo install*".into()],
    )
    .unwrap();
    assert!(matches!(
        m.evaluate(&argv(&["cargo", "install", "tokei"])),
        Decision::DeniedByPattern { .. }
    ));
    assert_eq!(m.evaluate(&argv(&["cargo", "build"])), Decision::Allow);
}

#[test]
fn star_in_middle_works() {
    let m = Matcher::new(&vec!["git *".into()], &vec!["git push*".into()]).unwrap();
    assert_eq!(m.evaluate(&argv(&["git", "status"])), Decision::Allow);
    assert!(matches!(
        m.evaluate(&argv(&["git", "push", "origin"])),
        Decision::DeniedByPattern { .. }
    ));
}

#[test]
fn pushable_table() {
    let m = Matcher::new(
        &vec![
            "just *".into(),
            "bun *".into(),
            "cargo check*".into(),
            "cargo build*".into(),
            "cargo test*".into(),
            "git status*".into(),
            "git diff*".into(),
        ],
        &vec![
            "git push*".into(),
            "rm -rf /*".into(),
            "curl *".into(),
            "wget *".into(),
            "sudo *".into(),
            "cargo install*".into(),
        ],
    )
    .unwrap();
    let allow = [
        vec!["just", "fast"],
        vec!["bun", "test"],
        vec!["cargo", "check"],
        vec!["cargo", "build", "--release"],
        vec!["cargo", "test", "-p", "sandboxctl"],
        vec!["git", "status"],
    ];
    for case in allow {
        let argv: Vec<String> = case.iter().map(|s| s.to_string()).collect();
        assert_eq!(m.evaluate(&argv), Decision::Allow, "expected allow: {argv:?}");
    }
    let deny = [
        vec!["git", "push"],
        vec!["rm", "-rf", "/"],
        vec!["curl", "https://evil"],
        vec!["sudo", "rm", "-rf"],
        vec!["cargo", "install", "x"],
    ];
    for case in deny {
        let argv: Vec<String> = case.iter().map(|s| s.to_string()).collect();
        let d = m.evaluate(&argv);
        assert!(
            matches!(d, Decision::DeniedByPattern { .. }),
            "expected denied: {argv:?} got {d:?}"
        );
    }
    let no_allow = [vec!["echo", "hi"], vec!["ls"], vec!["python", "-c", "1"]];
    for case in no_allow {
        let argv: Vec<String> = case.iter().map(|s| s.to_string()).collect();
        assert_eq!(
            m.evaluate(&argv),
            Decision::NoAllowMatched,
            "expected no-allow: {argv:?}"
        );
    }
}

proptest! {
    /// Property: deny patterns always win over allow patterns on overlapping
    /// argv. Generates a small alphabet so collisions are likely.
    #[test]
    fn deny_intersection_wins(
        allow_prefix in "[a-z]{1,4}",
        deny_subset in "[a-z]{1,6}",
        suffix in "[a-z]{0,4}",
    ) {
        let allow = format!("{allow_prefix} *");
        // Force the deny pattern to be a superset of allow (same prefix + more).
        let deny = format!("{allow_prefix} {deny_subset}*");
        let m = Matcher::new(&vec![allow], &vec![deny.clone()]).unwrap();
        let argv = vec![
            allow_prefix.clone(),
            format!("{deny_subset}{suffix}"),
        ];
        if let Decision::DeniedByPattern { pattern } = m.evaluate(&argv) {
            prop_assert_eq!(pattern, deny);
        } else {
            prop_assert!(false, "expected deny win for argv={argv:?}");
        }
    }
}
