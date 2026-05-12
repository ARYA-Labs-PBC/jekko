//! Deterministic FNV-1a 64-bit hash.
//!
//! We deliberately avoid std::collections::hash_map::DefaultHasher and any other
//! hasher whose output varies across Rust versions or build configurations.
//! FNV-1a is byte-deterministic and small enough to inline.

const FNV_OFFSET_64: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME_64: u64 = 0x0000_0100_0000_01B3;

/// FNV-1a hash of a byte slice.
#[inline]
pub fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut h = FNV_OFFSET_64;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME_64);
    }
    h
}

/// Compatibility alias for generated code that still expects the older name.
#[inline]
pub fn fnv1a_u64(bytes: &[u8]) -> u64 {
    fnv1a_64(bytes)
}

/// FNV-1a hash of a string, rendered as lowercase hex.
#[inline]
pub fn fnv1a_hex(s: &str) -> String {
    format!("{:016x}", fnv1a_64(s.as_bytes()))
}

/// Hash several string fragments in deterministic order.
pub fn fnv1a_seq_hex(parts: &[&str]) -> String {
    let mut h = FNV_OFFSET_64;
    for part in parts {
        // Length-prefix each part to avoid concatenation collisions.
        let len = part.len() as u64;
        for i in 0..8 {
            let b = ((len >> (i * 8)) & 0xff) as u8;
            h ^= b as u64;
            h = h.wrapping_mul(FNV_PRIME_64);
        }
        for &b in part.as_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(FNV_PRIME_64);
        }
    }
    format!("{:016x}", h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_empty() {
        assert_eq!(fnv1a_64(b""), FNV_OFFSET_64);
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
