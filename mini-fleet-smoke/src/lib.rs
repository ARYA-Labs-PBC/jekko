//! Mini fleet smoke crate.
//!
//! Tiny smoke surface used by the mini fleet harness. Ported from the
//! original TypeScript `math.ts` / `math.test.ts` pair so the workspace
//! can drop its last forbidden-runtime advisory hit.

/// Sums two signed 64-bit integers.
#[must_use]
pub fn sum(a: i64, b: i64) -> i64 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::sum;

    #[test]
    fn sum_adds() {
        assert_eq!(sum(2, 3), 5);
    }
}
