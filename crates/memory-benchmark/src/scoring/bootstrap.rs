use crate::generated::seed::SeedRng;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BootstrapCi {
    pub mean: f32,
    pub ci95_low: f32,
    pub ci95_high: f32,
}

pub fn bootstrap_ci(scores: &[f32], seed_label: &str, resamples: usize) -> BootstrapCi {
    if scores.is_empty() {
        return BootstrapCi {
            mean: 0.0,
            ci95_low: 0.0,
            ci95_high: 0.0,
        };
    }
    let mean = scores.iter().sum::<f32>() / scores.len() as f32;
    let mut rng = SeedRng::from_label(seed_label);
    let mut means = Vec::with_capacity(resamples.max(1));
    for _ in 0..resamples.max(1) {
        let mut total = 0.0;
        for _ in 0..scores.len() {
            let idx = (rng.next_u64() as usize) % scores.len();
            total += scores[idx];
        }
        means.push(total / scores.len() as f32);
    }
    means.sort_by(|a, b| a.total_cmp(b));
    let low_idx = ((means.len() - 1) as f32 * 0.025).round() as usize;
    let high_idx = ((means.len() - 1) as f32 * 0.975).round() as usize;
    BootstrapCi {
        mean,
        ci95_low: means[low_idx],
        ci95_high: means[high_idx],
    }
}
