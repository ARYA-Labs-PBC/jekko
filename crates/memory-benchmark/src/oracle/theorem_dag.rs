use std::collections::{BTreeMap, BTreeSet};

pub fn invalidated_closure(
    reverse_edges: &BTreeMap<String, Vec<String>>,
    roots: &[String],
) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut stack = roots.to_vec();
    while let Some(id) = stack.pop() {
        if !out.insert(id.clone()) {
            continue;
        }
        if let Some(children) = reverse_edges.get(&id) {
            for child in children {
                stack.push(child.clone());
            }
        }
    }
    out
}
