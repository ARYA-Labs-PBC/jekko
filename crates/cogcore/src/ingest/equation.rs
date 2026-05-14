//! Equation extraction from text — LaTeX-ish patterns + SI unit normalization.

/// Parsed equation atom. `units` is `Some` when a trailing `[unit]` was
/// recognized; the unit string is the canonical SI form (see
/// [`normalize_unit`]).
#[derive(Debug, Clone)]
pub struct EqAtom {
    /// Left-hand side identifier or expression token.
    pub lhs: String,
    /// Equation operator (`=`, `≈`, `∝`).
    pub op: String,
    /// Right-hand side expression (numeric or symbolic).
    pub rhs: String,
    /// Optional SI-normalized unit string.
    pub units: Option<String>,
}

/// Extract equations from a section's text. Recognizes patterns like:
///   `lhs = rhs [unit]`
///   `lhs ≈ rhs`
///   `lhs ∝ rhs`
///   `lhs = rhs` (no units)
///
/// Returns a Vec of EqAtom. Skips trivial fragments (empty lhs/rhs).
pub fn extract_equations(text: &str) -> Vec<EqAtom> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Find the next ASCII operator character. Multi-byte characters like
        // `≈` or `∝` start with a non-ASCII byte; we treat the multi-byte
        // sequence as a single operator if encountered.
        let mut op_idx: Option<usize> = None;
        let mut op_len: usize = 0;
        let mut op_str: String = String::new();
        let mut j = i;
        while j < bytes.len() {
            let c = bytes[j];
            if c == b'=' {
                op_idx = Some(j);
                op_len = 1;
                op_str = "=".to_string();
                break;
            }
            // UTF-8: 0xE2 begins `≈` (E2 89 88) and `∝` (E2 88 9D).
            if c == 0xE2 && j + 2 < bytes.len() {
                if bytes[j + 1] == 0x89 && bytes[j + 2] == 0x88 {
                    op_idx = Some(j);
                    op_len = 3;
                    op_str = "≈".to_string();
                    break;
                }
                if bytes[j + 1] == 0x88 && bytes[j + 2] == 0x9D {
                    op_idx = Some(j);
                    op_len = 3;
                    op_str = "∝".to_string();
                    break;
                }
            }
            j += 1;
        }
        let op_idx = match op_idx {
            Some(p) => p,
            None => break,
        };

        // LHS: scan backward from op_idx for an identifier token.
        let mut lhs_end = op_idx;
        while lhs_end > i && (bytes[lhs_end - 1] as char).is_whitespace() {
            lhs_end -= 1;
        }
        let mut lhs_start = lhs_end;
        while lhs_start > i {
            let c = bytes[lhs_start - 1] as char;
            if c.is_alphanumeric() || c == '_' || c == '^' {
                lhs_start -= 1;
            } else {
                break;
            }
        }
        if lhs_start >= lhs_end {
            i = op_idx + op_len;
            continue;
        }
        let lhs = text[lhs_start..lhs_end].to_string();

        // RHS: scan forward from after op until sentence punctuation. Stay
        // inside bracketed groups so unit annotations are retained.
        let mut rhs_start = op_idx + op_len;
        while rhs_start < bytes.len() && (bytes[rhs_start] as char).is_whitespace() {
            rhs_start += 1;
        }
        let mut rhs_end = rhs_start;
        let mut bracket_depth = 0;
        while rhs_end < bytes.len() {
            let c = bytes[rhs_end] as char;
            if c == '[' {
                bracket_depth += 1;
            }
            if c == ']' && bracket_depth > 0 {
                bracket_depth -= 1;
            }
            if bracket_depth == 0 && (c == '\n' || c == ',' || c == ';') {
                break;
            }
            // A '.' terminates the sentence unless it is a decimal point in a
            // number (digit on both sides, like the `.` in `7.5`).
            if bracket_depth == 0 && c == '.' {
                let prev_is_digit =
                    rhs_end > rhs_start && (bytes[rhs_end - 1] as char).is_ascii_digit();
                let next_is_digit =
                    rhs_end + 1 < bytes.len() && (bytes[rhs_end + 1] as char).is_ascii_digit();
                if !(prev_is_digit && next_is_digit) {
                    break;
                }
            }
            rhs_end += 1;
        }
        if rhs_end <= rhs_start {
            i = op_idx + op_len;
            continue;
        }
        let rhs_raw = text[rhs_start..rhs_end].trim().to_string();

        // Units: optional `[unit]` at end of rhs.
        let (rhs, units) = if let Some(lb) = rhs_raw.rfind('[') {
            if let Some(rb_rel) = rhs_raw[lb..].find(']') {
                let unit_str = &rhs_raw[lb + 1..lb + rb_rel];
                let rhs_main = rhs_raw[..lb].trim().to_string();
                (rhs_main, Some(normalize_unit(unit_str)))
            } else {
                (rhs_raw.clone(), None)
            }
        } else {
            (rhs_raw.clone(), None)
        };

        if !lhs.is_empty() && !rhs.is_empty() {
            out.push(EqAtom {
                lhs,
                op: op_str,
                rhs,
                units,
            });
        }
        i = rhs_end;
    }
    out
}

/// Canonicalize a unit string against a small SI table. Unknown units pass
/// through unchanged (after trimming).
fn normalize_unit(raw: &str) -> String {
    match raw.trim() {
        "eV" | "ev" => "eV".to_string(),
        "GeV" | "gev" => "GeV".to_string(),
        "MeV" | "mev" => "MeV".to_string(),
        "TeV" | "tev" => "TeV".to_string(),
        "kg" => "kg".to_string(),
        "m" => "m".to_string(),
        "s" => "s".to_string(),
        "m/s" => "m/s".to_string(),
        "J" => "J".to_string(),
        "W" => "W".to_string(),
        "K" => "K".to_string(),
        "mol" => "mol".to_string(),
        "cd" => "cd".to_string(),
        "A" => "A".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_simple_equation_with_units() {
        let text = "We measure delta m^2 = 7.5e-5 [eV^2]. The result is consistent.";
        let eqs = extract_equations(text);
        assert!(!eqs.is_empty());
        let eq = &eqs[0];
        // LHS scanner picks the longest identifier token preceding `=`.
        assert_eq!(eq.lhs, "m^2");
        assert_eq!(eq.op, "=");
        assert!(eq.rhs.contains("7.5e-5"));
        assert_eq!(eq.units.as_deref(), Some("eV^2"));
    }

    #[test]
    fn extracts_multiple_equations() {
        let text = "Energy E = mc^2. Mass m = 1.5 [kg]. Speed v = 0.99c.";
        let eqs = extract_equations(text);
        assert!(eqs.len() >= 2);
    }

    #[test]
    fn no_equation_returns_empty() {
        let text = "Plain text with no equations.";
        let eqs = extract_equations(text);
        assert!(eqs.is_empty());
    }
}
