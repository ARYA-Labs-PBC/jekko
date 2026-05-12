use memory_benchmark::oracle::privacy::scan_recall;
use memory_benchmark::{RecallResult, SkillCall};

#[test]
fn privacy_scanner_checks_output_channels() {
    let secret = "PRIVATE-1234".to_string();
    let mut result = RecallResult::default();
    result.answer = "redacted".to_string();
    result.skill_calls.push(SkillCall {
        name: "tool".to_string(),
        args_hash: "PRIVATE-1234".to_string(),
        refused: true,
    });
    let leaks = scan_recall(&result, &[secret]);
    assert!(!leaks.is_empty());
    assert_eq!(leaks[0].channel, "skill_calls");
}
