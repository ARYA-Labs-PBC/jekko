use memory_benchmark::generated::{
    generate_compounding_suite, generate_suite, names, seed::SeedRng, CompoundingConfig,
    GeneratedSuiteConfig,
};
use memory_benchmark::{Domain, Split};

#[test]
fn seed_rng_is_stable_for_known_label() {
    let mut rng = SeedRng::from_label("public-dev-0001");
    assert_eq!(rng.next_u64(), 0x3ce8a9cb19fa185f);
    assert_ne!(rng.next_u64(), 0x3ce8a9cb19fa185f);
}

#[test]
fn names_are_deterministic_and_brand_free() {
    let mut a = SeedRng::from_label("names");
    let mut b = SeedRng::from_label("names");
    let left = names::synthetic_name(&mut a, "case");
    let right = names::synthetic_name(&mut b, "case");
    assert_eq!(left, right);
    assert!(names::forbidden_brand_free(&left));
}

#[test]
fn generated_public_dev_count_and_order_are_stable() {
    let config = GeneratedSuiteConfig {
        benchmark_version: "memory-benchmark-v2",
        split: Split::PublicGenerated,
        seed_label: "public-dev-0001".to_string(),
        fixture_count: 500,
        difficulty: 2,
    };
    let cases = generate_suite(&config);
    assert_eq!(cases.len(), 500);
    assert_eq!(cases[0].id, "public-generated-00000");
    assert_eq!(cases[499].id, "public-generated-00499");
    for domain in [
        Domain::Math,
        Domain::Science,
        Domain::Privacy,
        Domain::Procedural,
    ] {
        assert!(cases.iter().filter(|case| case.domain == domain).count() >= 25);
    }
}

#[test]
fn compounding_suite_includes_real_paper_chain_kind() {
    let cases = generate_compounding_suite(&CompoundingConfig {
        benchmark_version: "memory-benchmark-v2",
        seed_label: "compound-test".to_string(),
        fixture_count: 14,
    });
    let case = cases
        .iter()
        .find(|case| case.id.ends_with("-real-paper"))
        .expect("real_paper_chain case");
    assert!(case.events.len() >= 4);
    assert!(case.queries.len() >= 2);
    assert!(case.queries.iter().any(|query| query.control));
    let primary = case
        .queries
        .iter()
        .find(|query| !query.control)
        .expect("primary query");
    assert_eq!(primary.hop_depth, 4);
    assert!(primary.depth_weight >= 3.4);
    assert!(primary
        .oracle
        .must_contain
        .iter()
        .any(|needle| needle == "contrastive section packing"));
}
