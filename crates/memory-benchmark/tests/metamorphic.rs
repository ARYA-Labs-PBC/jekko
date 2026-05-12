use memory_benchmark::generated::{generate_suite, metamorphic, GeneratedSuiteConfig};
use memory_benchmark::Split;

#[test]
fn metamorphic_rename_preserves_oracle() {
    let config = GeneratedSuiteConfig {
        benchmark_version: "memory-benchmark-v2",
        split: Split::PublicGenerated,
        seed_label: "public-dev-0001".to_string(),
        fixture_count: 1,
        difficulty: 2,
    };
    let case = generate_suite(&config).remove(0);
    let renamed = metamorphic::renamed(&case, "alpha");
    assert_eq!(renamed.oracle.must_contain, case.oracle.must_contain);
    assert_ne!(renamed.id, case.id);
}
