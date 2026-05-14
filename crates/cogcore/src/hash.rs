//! Deterministic FNV-1a 64-bit hash.
//!
//! Byte-stable across Rust versions and build configurations. Matches the
//! algorithm in `memory_benchmark::hash` so projection hashes remain
//! comparable when cogcore is wired as a benchmark candidate.

const FNV_OFFSET_64: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME_64: u64 = 0x0000_0100_0000_01B3;

#[inline]
pub fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut h = FNV_OFFSET_64;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME_64);
    }
    h
}

#[inline]
pub fn fnv1a_hex(s: &str) -> String {
    format!("{:016x}", fnv1a_64(s.as_bytes()))
}

pub fn fnv1a_seq_hex(parts: &[&str]) -> String {
    let mut h = FNV_OFFSET_64;
    for part in parts {
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
    fn matches_memory_benchmark_vectors() {
        assert_eq!(fnv1a_64(b""), FNV_OFFSET_64);
        assert_eq!(fnv1a_64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a_hex("hello"), "a430d84680aabd0b");
    }

    #[test]
    fn seq_hex_is_length_prefix_safe() {
        let h1 = fnv1a_seq_hex(&["ab", ""]);
        let h2 = fnv1a_seq_hex(&["a", "b"]);
        assert_ne!(h1, h2);
    }
}
