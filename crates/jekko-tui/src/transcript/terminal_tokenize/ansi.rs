//! ANSI escape-sequence stripping.

/// Strip ANSI CSI/OSC escape sequences from `text`. The result preserves byte
/// offsets within the visible portion so downstream tokens are stable. This
/// is hand-rolled (no `vte` dep yet — see the module notes).
pub fn strip_ansi(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == 0x1B && i + 1 < bytes.len() {
            // ESC
            let next = bytes[i + 1];
            match next {
                b'[' => {
                    // CSI: ESC [ ... final-byte (0x40..=0x7E)
                    i += 2;
                    while i < bytes.len() && !(0x40..=0x7E).contains(&bytes[i]) {
                        i += 1;
                    }
                    if i < bytes.len() {
                        i += 1;
                    }
                    continue;
                }
                b']' => {
                    // OSC: ESC ] ... BEL or ESC \
                    i += 2;
                    while i < bytes.len() {
                        if bytes[i] == 0x07 {
                            i += 1;
                            break;
                        }
                        if bytes[i] == 0x1B && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                    continue;
                }
                b'(' | b')' => {
                    // Charset selection: ESC ( X
                    i += 3.min(bytes.len() - i);
                    continue;
                }
                _ => {
                    // Skip lone ESC; drop the escape byte.
                    i += 1;
                    continue;
                }
            }
        }
        out.push(b);
        i += 1;
    }
    // `out` is built from valid UTF-8 segments only when the input was valid
    // UTF-8 with the escapes excised between codepoint boundaries; ANSI
    // sequences are ASCII so this holds in practice. Fall back to lossy on
    // edge cases.
    match String::from_utf8(out) {
        Ok(s) => s,
        Err(err) => {
            let bytes = err.into_bytes();
            String::from_utf8_lossy(&bytes).into_owned()
        }
    }
}
