use memory_benchmark::runner::run_candidate;

#[test]
fn candidate_lanes_are_not_all_baseline() {
    let baseline = run_candidate("baseline").expect("baseline").json;
    let ledger = run_candidate("ledger_first").expect("ledger").json;
    let temporal = run_candidate("temporal_graph").expect("temporal").json;
    assert_ne!(baseline, ledger);
    assert_ne!(ledger, temporal);
}
