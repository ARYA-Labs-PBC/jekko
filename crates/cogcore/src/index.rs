//! BM25-lite inverted index over interned tokens.
//!
//! Deterministic: tokens are assigned `TokenId`s in canonical sorted-bytes
//! order during `rebuild()`. Live insertion uses first-seen ordering but
//! the public surface (`top_k`) operates on sorted candidate lists so the
//! external output is invariant.

use std::collections::BTreeMap;

pub type TokenId = u32;

#[derive(Default)]
pub struct Interner {
    by_token: BTreeMap<String, TokenId>,
    by_id: Vec<String>,
}

impl Interner {
    pub fn intern(&mut self, token: &str) -> TokenId {
        if let Some(id) = self.by_token.get(token) {
            return *id;
        }
        let id = self.by_id.len() as TokenId;
        self.by_token.insert(token.to_string(), id);
        self.by_id.push(token.to_string());
        id
    }

    pub fn lookup(&self, token: &str) -> Option<TokenId> {
        self.by_token.get(token).copied()
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

/// Split body into lowercase ASCII-alphanumeric token runs.
pub fn tokenize(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            buf.push(c.to_ascii_lowercase());
        } else {
            if !buf.is_empty() {
                out.push(std::mem::take(&mut buf));
            }
        }
    }
    if !buf.is_empty() {
        out.push(buf);
    }
    out
}

/// Token-bigram set for MinHash and Jaccard.
pub fn bigrams(tokens: &[TokenId]) -> Vec<u64> {
    if tokens.len() < 2 {
        return tokens.iter().map(|t| *t as u64).collect();
    }
    let mut out = Vec::with_capacity(tokens.len() - 1);
    for w in tokens.windows(2) {
        let pair = ((w[0] as u64) << 32) | (w[1] as u64);
        out.push(pair);
    }
    out.sort();
    out.dedup();
    out
}

/// 8-hash MinHash sketch — deterministic seeds.
pub const MINHASH_SEEDS: [u64; 8] = [
    0xdead_beef_dead_beef,
    0xfeed_face_feed_face,
    0xcafe_babe_cafe_babe,
    0xbaad_f00d_baad_f00d,
    0x1234_5678_1234_5678,
    0x8765_4321_8765_4321,
    0xa5a5_a5a5_a5a5_a5a5,
    0x5a5a_5a5a_5a5a_5a5a,
];

pub fn minhash_sketch(items: &[u64]) -> [u32; 8] {
    let mut sketch = [u32::MAX; 8];
    if items.is_empty() {
        return sketch;
    }
    for (i, seed) in MINHASH_SEEDS.iter().enumerate() {
        for &x in items {
            let h = mix64(x ^ *seed) as u32;
            if h < sketch[i] {
                sketch[i] = h;
            }
        }
    }
    sketch
}

fn mix64(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

pub fn jaccard_minhash(a: &[u32; 8], b: &[u32; 8]) -> f32 {
    let matches = a.iter().zip(b.iter()).filter(|(x, y)| x == y).count();
    matches as f32 / 8.0
}

/// Posting list entry: (CellIndex, term frequency).
#[derive(Clone, Default)]
pub struct InvertedIndex {
    /// token -> sorted Vec<(cell_idx, tf)>
    pub postings: BTreeMap<TokenId, Vec<(u32, u16)>>,
    /// per-cell document length (token count)
    pub doc_len: Vec<u32>,
    /// running average doc length for BM25 normalization
    pub avg_doc_len: f32,
    /// per-cell sorted unique TokenId list (for Jaccard / topic ops)
    pub doc_tokens: Vec<Vec<TokenId>>,
}

impl InvertedIndex {
    /// Add a document. Returns the assigned cell index.
    pub fn add(&mut self, tokens: &[TokenId]) -> u32 {
        let cell_idx = self.doc_len.len() as u32;
        let mut counts: BTreeMap<TokenId, u16> = BTreeMap::new();
        for t in tokens {
            *counts.entry(*t).or_insert(0) += 1;
        }
        let doc_len = tokens.len() as u32;
        for (tok, tf) in counts {
            self.postings.entry(tok).or_default().push((cell_idx, tf));
        }
        let mut sorted_tokens: Vec<TokenId> = tokens.to_vec();
        sorted_tokens.sort();
        sorted_tokens.dedup();
        self.doc_tokens.push(sorted_tokens);
        let prior_count = self.doc_len.len() as f32;
        self.doc_len.push(doc_len);
        let new_count = self.doc_len.len() as f32;
        self.avg_doc_len = (self.avg_doc_len * prior_count + doc_len as f32) / new_count.max(1.0);
        cell_idx
    }

    /// BM25 score for a single query against a cell.
    pub fn bm25(&self, query_tokens: &[TokenId], cell_idx: u32) -> f32 {
        let k1 = 1.5_f32;
        let b = 0.75_f32;
        let total_docs = self.doc_len.len() as f32;
        if total_docs == 0.0 {
            return 0.0;
        }
        let dl = self.doc_len.get(cell_idx as usize).copied().unwrap_or(0) as f32;
        let avg = self.avg_doc_len.max(1.0);
        let mut score = 0.0;
        for q in query_tokens {
            let Some(postings) = self.postings.get(q) else {
                continue;
            };
            let df = postings.len() as f32;
            let idf = ((total_docs - df + 0.5) / (df + 0.5) + 1.0).ln();
            if let Ok(pos) = postings.binary_search_by_key(&cell_idx, |(c, _)| *c) {
                let tf = postings[pos].1 as f32;
                let denom = tf + k1 * (1.0 - b + b * dl / avg);
                score += idf * (tf * (k1 + 1.0)) / denom.max(1e-6);
            }
        }
        score
    }

    /// Return cell indices with at least one query token in posting list.
    pub fn candidate_cells(&self, query_tokens: &[TokenId], cap: usize) -> Vec<u32> {
        let mut hits: BTreeMap<u32, u32> = BTreeMap::new();
        for q in query_tokens {
            if let Some(postings) = self.postings.get(q) {
                for (cell_idx, _tf) in postings {
                    *hits.entry(*cell_idx).or_insert(0) += 1;
                }
            }
        }
        let mut sorted: Vec<(u32, u32)> = hits.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        sorted.into_iter().take(cap).map(|(c, _)| c).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_lowercases_and_splits() {
        let t = tokenize("Neutrino-Mass IS 0.1 eV");
        assert_eq!(t, vec!["neutrino", "mass", "is", "0", "1", "ev"]);
    }

    #[test]
    fn bm25_higher_for_better_match() {
        let mut int = Interner::default();
        let mut idx = InvertedIndex::default();
        let toks1: Vec<TokenId> = tokenize("neutrino mass oscillation")
            .iter()
            .map(|t| int.intern(t))
            .collect();
        let toks2: Vec<TokenId> = tokenize("muon decay rate")
            .iter()
            .map(|t| int.intern(t))
            .collect();
        idx.add(&toks1);
        idx.add(&toks2);
        let q: Vec<TokenId> = tokenize("neutrino oscillation")
            .iter()
            .filter_map(|t| int.lookup(t))
            .collect();
        let s1 = idx.bm25(&q, 0);
        let s2 = idx.bm25(&q, 1);
        assert!(s1 > s2);
    }

    #[test]
    fn minhash_jaccard_estimates() {
        let mut int = Interner::default();
        let toks1: Vec<TokenId> = tokenize("a b c d e f g h")
            .iter()
            .map(|t| int.intern(t))
            .collect();
        let toks2: Vec<TokenId> = tokenize("a b c d e f g h")
            .iter()
            .map(|t| int.intern(t))
            .collect();
        let bg1 = bigrams(&toks1);
        let bg2 = bigrams(&toks2);
        let sk1 = minhash_sketch(&bg1);
        let sk2 = minhash_sketch(&bg2);
        assert_eq!(jaccard_minhash(&sk1, &sk2), 1.0);
    }
}
