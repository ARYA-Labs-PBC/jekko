//! Proof crate for the Jekko unlock PTY flow.

/// Keep a tiny public surface so the crate has a concrete library target.
pub fn crate_name() -> &'static str {
    "tuiwright-jekko-unlock"
}

#[cfg(test)]
mod tests {
    use super::crate_name;

    #[test]
    fn reports_the_crate_name() {
        assert_eq!(crate_name(), "tuiwright-jekko-unlock");
    }
}
