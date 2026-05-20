/// Best-effort PATH lookup. Returns `Some(absolute path)` if `program` resolves
/// in any `$PATH` entry. Avoids pulling in a new crate just for one check.
fn which_in_path(program: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(program);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn env_or(name: &str, missing: &str) -> String {
    match std::env::var(name) {
        Ok(value) => value,
        Err(_) => missing.to_string(),
    }
}

fn current_dir_or_dot() -> PathBuf {
    match std::env::current_dir() {
        Ok(path) => path,
        Err(_) => PathBuf::from("."),
    }
}

fn chunk_string(s: &str, max: usize) -> Vec<&str> {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut start = 0;
    while start < bytes.len() {
        let mut end = (start + max).min(bytes.len());
        while end < bytes.len() && (bytes[end] & 0xC0) == 0x80 {
            end += 1;
        }
        out.push(&s[start..end]);
        start = end;
    }
    out
}

/// Construct a writer that the panic hook can use to restore the terminal
/// before printing the panic message. Mainly exists so callers don't have to
/// reach into `lifecycle::*` directly.
pub fn stdout_for_panic_restore() -> impl Write {
    io::stdout()
}
