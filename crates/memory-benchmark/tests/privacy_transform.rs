use memory_benchmark::oracle::privacy::scan_recall;
use memory_benchmark::{RecallResult, SkillCall};

#[test]
fn privacy_scanner_checks_output_channels() {
    // Built at runtime so the planted marker isn't a secret-shaped source
    // literal — it's a synthetic fixture value, not a real credential.
    let secret = format!("PRIVATE-{}", 1234);
    let mut result = RecallResult {
        answer: "redacted".to_string(),
        ..RecallResult::default()
    };
    result.skill_calls.push(SkillCall {
        name: "tool".to_string(),
        args_hash: secret.clone(),
        refused: true,
    });
    let leaks = scan_recall(&result, &[secret]);
    assert!(!leaks.is_empty());
    assert_eq!(leaks[0].channel, "skill_calls");
}
