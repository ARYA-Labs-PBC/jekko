use super::env_info::{current_host_triple, resolve_version};
use super::flags::parse_flags;
use super::plan::{bun_target_to_rust_triple, is_host_target};
use super::types::PublishBuildTarget;
use std::env;

#[test]
fn parse_flags_recognises_all_known_tokens() {
    let f = parse_flags(&["--single", "--baseline", "--skip-install", "--sourcemaps"]);
    assert!(f.single);
    assert!(f.baseline);
    assert!(f.skip_install);
    assert!(f.sourcemaps);
}

#[test]
fn parse_flags_defaults_to_all_false() {
    let f = parse_flags(&[]);
    assert!(!f.single);
    assert!(!f.baseline);
}

#[test]
fn bun_target_to_rust_triple_covers_known_platforms() {
    assert_eq!(
        bun_target_to_rust_triple("bun-darwin-arm64"),
        Some("aarch64-apple-darwin")
    );
    assert_eq!(
        bun_target_to_rust_triple("bun-darwin-x64"),
        Some("x86_64-apple-darwin")
    );
    assert_eq!(
        bun_target_to_rust_triple("bun-linux-arm64"),
        Some("aarch64-unknown-linux-gnu")
    );
    assert_eq!(
        bun_target_to_rust_triple("bun-linux-x64"),
        Some("x86_64-unknown-linux-gnu")
    );
    assert_eq!(
        bun_target_to_rust_triple("bun-linux-arm64-musl"),
        Some("aarch64-unknown-linux-musl")
    );
    assert_eq!(
        bun_target_to_rust_triple("bun-linux-x64-musl"),
        Some("x86_64-unknown-linux-musl")
    );
    assert_eq!(
        bun_target_to_rust_triple("bun-windows-x64"),
        Some("x86_64-pc-windows-msvc")
    );
    assert_eq!(
        bun_target_to_rust_triple("bun-windows-arm64"),
        Some("aarch64-pc-windows-msvc")
    );
}

#[test]
fn bun_target_to_rust_triple_handles_baseline_suffix() {
    assert_eq!(
        bun_target_to_rust_triple("bun-linux-x64-baseline"),
        Some("x86_64-unknown-linux-gnu")
    );
    assert_eq!(
        bun_target_to_rust_triple("bun-darwin-x64-baseline"),
        Some("x86_64-apple-darwin")
    );
    assert_eq!(
        bun_target_to_rust_triple("bun-windows-x64-baseline"),
        Some("x86_64-pc-windows-msvc")
    );
}

#[test]
fn bun_target_to_rust_triple_returns_none_for_unknown() {
    assert_eq!(bun_target_to_rust_triple("bun-haiku-riscv64"), None);
    assert_eq!(bun_target_to_rust_triple(""), None);
}

#[test]
fn resolve_version_falls_back_when_env_unset() {
    // SAFETY: tests in this binary are not multithreaded.
    let prev = env::var("JEKKO_VERSION").ok();
    env::remove_var("JEKKO_VERSION");
    let version = resolve_version();
    if let Some(value) = prev {
        env::set_var("JEKKO_VERSION", value);
    }
    assert_eq!(version, "0.0.0-local");
}

#[test]
fn is_host_target_skips_baseline_and_musl() {
    let baseline = PublishBuildTarget {
        os: "darwin".into(),
        arch: "arm64".into(),
        name: "jekko-darwin-arm64-baseline".into(),
        bun_target: "bun-darwin-arm64-baseline".into(),
        abi: None,
        avx2: Some(false),
    };
    assert!(!is_host_target(&baseline));

    let musl = PublishBuildTarget {
        os: "linux".into(),
        arch: "x64".into(),
        name: "jekko-linux-x64-musl".into(),
        bun_target: "bun-linux-x64-musl".into(),
        abi: Some("musl".into()),
        avx2: None,
    };
    assert!(!is_host_target(&musl));
}

#[test]
fn current_host_triple_is_non_empty_on_supported_platforms() {
    // Test runs on a real OS/arch; the mapping table covers all our
    // supported build hosts.
    let triple = current_host_triple();
    assert!(!triple.is_empty(), "host triple mapping missing");
}
