pub fn shortest_safe_workflow(steps: &[&str]) -> Option<Vec<String>> {
    let mut out = Vec::new();
    for step in steps {
        if matches!(*step, "net_exfil" | "fs_delete" | "unsafe_exec") {
            return None;
        }
        out.push((*step).to_string());
    }
    Some(out)
}
