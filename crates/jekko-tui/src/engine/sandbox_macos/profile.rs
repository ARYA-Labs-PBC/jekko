/// Render a TinyScheme/SBPL profile that enforces `policy`.
///
/// The function never panics; missing fields (e.g. `policy.cwd = None`)
/// simply emit no corresponding rule. This means an empty policy renders
/// a profile that allows almost nothing — callers that don't want enforcement
/// should pass `None` for the policy to the runner, not an empty policy.
pub fn render_profile(policy: &SandboxPolicy) -> String {
    let mut out = String::new();
    out.push_str("(version 1)\n");
    out.push_str("(deny default)\n");
    out.push('\n');

    out.push_str("(allow process-exec*)\n");
    out.push_str("(allow process-fork)\n");
    out.push('\n');

    out.push_str("(allow file-read*\n");
    out.push_str("  (literal \"/\")\n");
    if let Some(cwd) = policy.cwd.as_ref() {
        push_subpath_pair(&mut out, cwd);
    }
    for p in &policy.allowed_paths {
        push_subpath_pair(&mut out, p);
    }
    out.push_str("  (subpath \"/usr/lib\")\n");
    out.push_str("  (subpath \"/usr/share\")\n");
    out.push_str("  (subpath \"/System/Library\")\n");
    out.push_str("  (subpath \"/Library/Frameworks\")\n");
    out.push_str("  (subpath \"/private/var/db/dyld\")\n");
    out.push_str("  (literal \"/dev/null\")\n");
    out.push_str("  (literal \"/dev/random\")\n");
    out.push_str("  (literal \"/dev/urandom\")\n");
    out.push_str("  (literal \"/dev/tty\"))\n");
    out.push('\n');

    out.push_str("(allow file-read-metadata)\n");
    out.push('\n');

    out.push_str("(allow file-ioctl\n");
    out.push_str("  (literal \"/dev/dtracehelper\"))\n");
    out.push('\n');

    out.push_str("(allow file-write*\n");
    if let Some(cwd) = policy.cwd.as_ref() {
        push_subpath_pair(&mut out, cwd);
    }
    out.push_str("  (literal \"/dev/null\")\n");
    out.push_str("  (literal \"/dev/tty\"))\n");
    out.push('\n');

    out.push_str("(allow mach-lookup)\n");
    out.push('\n');

    out.push_str("(allow sysctl-read)\n");
    out.push('\n');

    if policy.allow_net {
        out.push_str("(allow network*)\n");
    } else {
        out.push_str("(deny network*)\n");
    }

    out
}
