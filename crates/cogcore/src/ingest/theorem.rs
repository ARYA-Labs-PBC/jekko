//! Theorem header extraction — looks for "Theorem X (...)?: ..." patterns.

/// Parsed reference to a theorem (or lemma / proposition / corollary).
/// `dependencies` records other theorem names referenced inside the
/// statement; this scaffolds a dependency DAG used downstream.
#[derive(Debug, Clone)]
pub struct TheoremRef {
    /// Header keyword: `Theorem` / `Lemma` / `Proposition` / `Corollary`.
    pub kind: String,
    /// Identifier following the keyword (e.g. `3.1`, `A`, `Bell`).
    pub name: String,
    /// Body of the theorem statement (between the colon/period and the
    /// next sentence boundary).
    pub statement: String,
    /// Referenced theorem names parsed from the statement (e.g.
    /// `Theorem 3.1`, `Lemma A`).
    pub dependencies: Vec<String>,
}

/// Header keywords recognized by the extractor. Order matters only for
/// determinism: the output is grouped by keyword in `KEYWORDS` order.
const KEYWORDS: &[&str] = &["Theorem", "Lemma", "Proposition", "Corollary"];

/// Extract every theorem-like header from the supplied text.
pub fn extract_theorems(text: &str) -> Vec<TheoremRef> {
    let mut out = Vec::new();
    for keyword in KEYWORDS {
        let mut i = 0;
        while let Some(pos) = text[i..].find(keyword) {
            let abs_pos = i + pos;
            // Must appear at a sentence boundary: either the very start of the
            // text, or preceded by whitespace immediately following one of
            // `.`, `\n`, `:` (so the keyword is actually a header rather than
            // a body mention like "by Theorem 3.1").
            let is_header_position = if abs_pos == 0 {
                true
            } else {
                let bytes = text.as_bytes();
                let mut k = abs_pos;
                while k > 0 && (bytes[k - 1] as char).is_whitespace() {
                    k -= 1;
                }
                k == 0 || matches!(bytes[k - 1] as char, '.' | '\n' | ':')
            };
            if !is_header_position {
                i = abs_pos + keyword.len();
                continue;
            }
            // Must be followed by whitespace + identifier.
            let after = abs_pos + keyword.len();
            if after >= text.len() {
                i = after;
                continue;
            }
            let ch = text.as_bytes()[after] as char;
            if !ch.is_whitespace() {
                i = after;
                continue;
            }
            // Extract name (identifier or number).
            let mut j = after + 1;
            while j < text.len() && (text.as_bytes()[j] as char).is_whitespace() {
                j += 1;
            }
            let name_start = j;
            while j < text.len() {
                let c = text.as_bytes()[j] as char;
                if c.is_alphanumeric() || c == '.' || c == '_' || c == '-' {
                    j += 1;
                } else {
                    break;
                }
            }
            let name = text[name_start..j].to_string();
            if name.is_empty() {
                i = after;
                continue;
            }
            // Statement: from `:` / `.` / `—` up to the next sentence boundary.
            let mut statement_start = j;
            while statement_start < text.len() {
                let c = text.as_bytes()[statement_start] as char;
                if c == ':' || c == '.' || c == '—' {
                    statement_start += 1;
                    break;
                }
                statement_start += 1;
            }
            let mut statement_end = statement_start;
            while statement_end < text.len() {
                let c = text.as_bytes()[statement_end] as char;
                if c == '\n' {
                    break;
                }
                if c == '.' {
                    // Decimal points inside numeric identifiers (e.g. the `.`
                    // in `3.1`) do not terminate the statement.
                    let bytes = text.as_bytes();
                    let prev_is_digit = statement_end > statement_start
                        && (bytes[statement_end - 1] as char).is_ascii_digit();
                    let next_is_digit = statement_end + 1 < bytes.len()
                        && (bytes[statement_end + 1] as char).is_ascii_digit();
                    if !(prev_is_digit && next_is_digit) {
                        break;
                    }
                }
                statement_end += 1;
            }
            let statement = text[statement_start..statement_end].trim().to_string();
            // Dependencies: scan the statement for `Theorem X` / `Lemma Y`.
            let mut deps = Vec::new();
            for dep_kw in KEYWORDS {
                let mut dpos = 0;
                while let Some(p) = statement[dpos..].find(dep_kw) {
                    let absp = dpos + p + dep_kw.len();
                    if absp < statement.len()
                        && (statement.as_bytes()[absp] as char).is_whitespace()
                    {
                        let mut k = absp + 1;
                        while k < statement.len()
                            && (statement.as_bytes()[k] as char).is_whitespace()
                        {
                            k += 1;
                        }
                        let dn_start = k;
                        while k < statement.len() {
                            let c = statement.as_bytes()[k] as char;
                            if c.is_alphanumeric() || c == '.' || c == '_' || c == '-' {
                                k += 1;
                            } else {
                                break;
                            }
                        }
                        let dep_name = statement[dn_start..k].to_string();
                        if !dep_name.is_empty() && dep_name != name {
                            deps.push(format!("{} {}", dep_kw, dep_name));
                        }
                    }
                    dpos += p + 1;
                }
            }
            out.push(TheoremRef {
                kind: keyword.to_string(),
                name,
                statement,
                dependencies: deps,
            });
            i = statement_end;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_simple_theorem() {
        let text = "Theorem 3.1: Every cell has a unique id.";
        let ts = extract_theorems(text);
        assert_eq!(ts.len(), 1);
        assert_eq!(ts[0].kind, "Theorem");
        assert_eq!(ts[0].name, "3.1");
        assert!(ts[0].statement.contains("unique id"));
    }

    #[test]
    fn extracts_dependency() {
        let text = "Corollary A: This follows by Theorem 3.1.";
        let ts = extract_theorems(text);
        assert_eq!(ts.len(), 1);
        assert!(ts[0].dependencies.iter().any(|d| d.contains("3.1")));
    }

    #[test]
    fn multiple_theorems() {
        let text = "Theorem 1: First. Lemma 2: Second. Proposition 3: Third.";
        let ts = extract_theorems(text);
        assert_eq!(ts.len(), 3);
    }
}
