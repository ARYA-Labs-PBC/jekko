use std::collections::{BTreeMap, BTreeSet};

use memory_benchmark::oracle::theorem_dag::invalidated_closure;

#[test]
fn theorem_dag_closure_walks_dependents() {
    let mut reverse = BTreeMap::new();
    reverse.insert("axiom".to_string(), vec!["lemma".to_string()]);
    reverse.insert("lemma".to_string(), vec!["theorem".to_string()]);
    let roots = vec!["axiom".to_string()];
    let got = invalidated_closure(&reverse, &roots);
    let expected = ["axiom", "lemma", "theorem"]
        .iter()
        .map(|s| s.to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(got, expected);
}
