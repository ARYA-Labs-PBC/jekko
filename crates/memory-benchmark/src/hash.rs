//! Deterministic FNV-1a 64-bit hash.
//!
//! This crate intentionally reuses the canonical `cogcore` implementation so
//! the benchmark harness and core runtime stay byte-for-byte aligned.

use cogcore::hash as cogcore_hash;

/// FNV-1a hash of a byte slice.
#[inline]
pub fn fnv1a_64(bytes: &[u8]) -> u64 {
    cogcore_hash::fnv1a_64(bytes)
}

/// Compatibility alias for generated code that still expects the older name.
#[inline]
pub fn fnv1a_u64(bytes: &[u8]) -> u64 {
    fnv1a_64(bytes)
}

/// FNV-1a hash of a string, rendered as lowercase hex.
#[inline]
pub fn fnv1a_hex(s: &str) -> String {
    cogcore_hash::fnv1a_hex(s)
}

/// Hash several string fragments in deterministic order.
#[inline]
pub fn fnv1a_seq_hex(parts: &[&str]) -> String {
    cogcore_hash::fnv1a_seq_hex(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_empty() {
        assert_eq!(fnv1a_64(b""), cogcore_hash::fnv1a_64(b""));
    }

    #[test]
    fn fnv1a_known_values() {
        // Standard FNV-1a test vectors.
        assert_eq!(fnv1a_64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a_64(b"foobar"), 0x85_944171f73967e8);
    }

    #[test]
    fn fnv1a_hex_is_16_chars() {
        let h = fnv1a_hex("hello");
        assert_eq!(h.len(), 16);
        assert_eq!(h, "a430d84680aabd0b");
    }

    #[test]
    fn fnv1a_seq_distinguishes_concat() {
        // "ab" + "" should differ from "a" + "b".
        let h1 = fnv1a_seq_hex(&["ab", ""]);
        let h2 = fnv1a_seq_hex(&["a", "b"]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn fnv1a_deterministic_across_invocations() {
        let a = fnv1a_hex("MEMORY_BENCH-deterministic-seed");
        let b = fnv1a_hex("MEMORY_BENCH-deterministic-seed");
        assert_eq!(a, b);
    }
}
