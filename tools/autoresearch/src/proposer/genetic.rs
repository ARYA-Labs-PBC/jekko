//! Deterministic genetic-algorithm proposer (T1 lane).
//!
//! Phase 4 ships hyperparameter sweeps over `seed_label`. The proposer is
//! seeded by `(cycle_id, worker_id, parent_seed)` and produces a small
//! set of candidate seeds plus a bounded `config.rs` patch for the
//! worktree lane.

use crate::template::render_config_patch;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Proposal {
    pub worker_id: u32,
    pub seed_label: String,
    pub tier: &'static str,
    pub gauss: f32,
    pub patch_path: &'static str,
    pub patch_content: String,
}

pub fn propose(workers: usize, cycle_id: &str, parent_seed: &str) -> Vec<Proposal> {
    let mut out = Vec::with_capacity(workers);
    for worker in 0..workers {
        let g = gauss_from_label(&format!("{cycle_id}:{worker}:{parent_seed}"));
        // Deterministic, bounded perturbation of parent_seed via numeric
        // suffix. Bench treats seed_label as opaque so any UTF-8 works.
        let perturbed = format!("{parent_seed}-c{cycle_id}-w{worker:02}-g{:+.2}", g);
        out.push(Proposal {
            worker_id: worker as u32,
            seed_label: perturbed,
            tier: "T1",
            gauss: g,
            patch_path: "crates/cogcore/src/config.rs",
            patch_content: render_config_patch(worker as u32, cycle_id, g),
        });
    }
    out
}

#[allow(dead_code)]
pub fn to_json(p: &Proposal) -> String {
    format!(
        "{{\"worker_id\":{},\"tier\":\"{}\",\"seed_label\":\"{}\",\"gauss\":{:.4},\"patch_path\":\"{}\"}}\n",
        p.worker_id, p.tier, p.seed_label, p.gauss, p.patch_path
    )
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h = FNV_OFFSET;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

fn gauss_from_label(label: &str) -> f32 {
    // Box-Muller from two independent FNV-1a digests; deterministic and
    // never touches the clock. sigma = 0.15.
    let u1 = (fnv1a(label.as_bytes()) as f32) / (u64::MAX as f32);
    let u2 = (fnv1a(format!("{label}/2").as_bytes()) as f32) / (u64::MAX as f32);
    let u1 = u1.max(1e-6);
    let mag = (-2.0 * u1.ln()).sqrt();
    let z0 = mag * (2.0 * std::f32::consts::PI * u2).cos();
    z0 * 0.15
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proposals_are_deterministic() {
        let a = propose(4, "0000001", "public-dev-0001");
        let b = propose(4, "0000001", "public-dev-0001");
        for (p, q) in a.iter().zip(b.iter()) {
            assert_eq!(p.seed_label, q.seed_label);
            assert!((p.gauss - q.gauss).abs() < 1e-6);
        }
    }
}
