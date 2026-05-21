/// Append a `(subpath "...")` clause with two-space indent. Skips empty
/// or whitespace-only paths defensively; the wrapper would reject the
/// profile outright in that case.
fn push_subpath(out: &mut String, p: &Path) {
    let display = p.display().to_string();
    if display.trim().is_empty() {
        return;
    }
    let escaped = display.replace('\\', "\\\\").replace('"', "\\\"");
    out.push_str("  (subpath \"");
    out.push_str(&escaped);
    out.push_str("\")\n");
}

/// Append a `(subpath ...)` clause for both the literal path AND its
/// canonicalized form. On macOS `/tmp` is a symlink to `/private/tmp`
/// and `/var` to `/private/var`; the kernel checks the canonical path
/// against the profile, so a profile entry for `/tmp/foo` does not match
/// a real syscall on `/private/tmp/foo`. Emitting both forms lets callers
/// pass either path style without surprises.
fn push_subpath_pair(out: &mut String, p: &Path) {
    push_subpath(out, p);
    if let Ok(canon) = std::fs::canonicalize(p) {
        if canon != *p {
            push_subpath(out, &canon);
        }
    }
}
