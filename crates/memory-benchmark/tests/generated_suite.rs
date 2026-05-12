use memory_benchmark::generated::{generate_suite, names, seed::SeedRng, GeneratedSuiteConfig};
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
