//! Sparse Hebbian co-activation matrix.
//!
//! Stores `(min(a,b), max(a,b)) -> weight` so the matrix is symmetric by
//! construction. Updates are deterministic given the input pair list.

use std::collections::BTreeMap;

use crate::config::{
    HEBB_CAP_PAIRS, HEBB_ETA_FAILURE, HEBB_ETA_FALSIFY, HEBB_ETA_IGNORE, HEBB_ETA_RECALL,
    HEBB_ETA_SUCCESS, HEBB_PRUNE_BELOW,
};

#[derive(Default)]
pub struct Hebb {
    coact: BTreeMap<(u32, u32), f32>,
}

impl Hebb {
    pub fn weight(&self, a: u32, b: u32) -> f32 {
        let key = order(a, b);
        self.coact.get(&key).copied().unwrap_or(0.0)
    }

    pub fn update_recall(&mut self, used: &[u32]) {
        self.apply_positive(used, HEBB_ETA_RECALL, HEBB_CAP_PAIRS);
    }

    pub fn update_success(&mut self, used: &[u32]) {
        self.apply_positive(used, HEBB_ETA_SUCCESS, HEBB_CAP_PAIRS);
    }

    pub fn update_falsify(&mut self, used: &[u32]) {
        self.apply_negative(used, HEBB_ETA_FALSIFY, HEBB_CAP_PAIRS);
    }

    pub fn update_failure(&mut self, used: &[u32]) {
        self.apply_negative(used, HEBB_ETA_FAILURE, HEBB_CAP_PAIRS);
    }

    pub fn update_ignore(&mut self, used: &[u32]) {
        self.apply_negative(used, HEBB_ETA_IGNORE, HEBB_CAP_PAIRS);
    }

    fn apply_positive(&mut self, used: &[u32], eta: f32, cap_pairs: usize) {
        let mut count = 0usize;
        for (i, a) in used.iter().enumerate() {
            for b in &used[i + 1..] {
                if count >= cap_pairs {
                    return;
                }
                let key = order(*a, *b);
                let prev = self.coact.get(&key).copied().unwrap_or(0.0);
                self.coact.insert(key, prev + eta * (1.0 - prev));
                count += 1;
            }
        }
    }

    fn apply_negative(&mut self, used: &[u32], eta: f32, cap_pairs: usize) {
        let mut count = 0usize;
        for (i, a) in used.iter().enumerate() {
            for b in &used[i + 1..] {
                if count >= cap_pairs {
                    return;
                }
                let key = order(*a, *b);
                if let Some(prev) = self.coact.get(&key).copied() {
                    self.coact.insert(key, (prev - eta * prev).max(0.0));
                }
                count += 1;
            }
        }
    }

    /// Sum of edge weights from `cell` to any cell in `pool` (sans self).
    pub fn boost_against(&self, cell: u32, pool: &[u32]) -> f32 {
        let mut sum = 0.0;
        for &other in pool {
            if other == cell {
                continue;
            }
            let key = order(cell, other);
            if let Some(w) = self.coact.get(&key) {
                sum += *w;
            }
        }
        sum
    }

    /// Sorted iteration of edges for state hashing.
    pub fn edges_sorted(&self) -> impl Iterator<Item = (&(u32, u32), &f32)> {
        self.coact.iter()
    }

    pub fn prune(&mut self) {
        self.coact.retain(|_, w| *w >= HEBB_PRUNE_BELOW);
    }

    pub fn len(&self) -> usize {
        self.coact.len()
    }

    pub fn is_empty(&self) -> bool {
        self.coact.is_empty()
    }
}

#[inline]
fn order(a: u32, b: u32) -> (u32, u32) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recall_increments_then_success_increments_more() {
        let mut h = Hebb::default();
        h.update_recall(&[1, 2, 3]);
        let w_after_recall = h.weight(1, 2);
        h.update_success(&[1, 2]);
        assert!(h.weight(1, 2) > w_after_recall);
    }

    #[test]
    fn falsify_decreases() {
        let mut h = Hebb::default();
        h.update_success(&[1, 2]);
        let before = h.weight(1, 2);
        h.update_falsify(&[1, 2]);
        assert!(h.weight(1, 2) < before);
    }

    #[test]
    fn weight_is_symmetric() {
        let mut h = Hebb::default();
        h.update_recall(&[5, 3]);
        assert_eq!(h.weight(3, 5), h.weight(5, 3));
    }
}
