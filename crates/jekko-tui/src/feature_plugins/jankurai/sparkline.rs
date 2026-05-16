//! Unicode-block sparkline renderer.

pub(super) const GLYPHS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
pub(super) const BLANK_GLYPH: char = '·';

/// Unicode-block sparkline renderer. Ports `sparkline.ts::sparkline`.
///
/// Tails to the right so the most recent samples always appear; pads with
/// blank glyphs when there are fewer samples than `width`.
pub fn sparkline(values: &[f64], width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if values.is_empty() {
        return BLANK_GLYPH.to_string().repeat(width);
    }
    let tail: Vec<f64> = if values.len() > width {
        values[values.len() - width..].to_vec()
    } else {
        values.to_vec()
    };
    let finite: Vec<f64> = tail.iter().copied().filter(|v| v.is_finite()).collect();
    if finite.is_empty() {
        return BLANK_GLYPH.to_string().repeat(width);
    }
    let mut min = finite[0];
    let mut max = finite[0];
    for v in &finite {
        if *v < min {
            min = *v;
        }
        if *v > max {
            max = *v;
        }
    }
    let span = max - min;
    let mut out: String = tail
        .iter()
        .map(|v| {
            if !v.is_finite() {
                return BLANK_GLYPH;
            }
            if span == 0.0 {
                return GLYPHS[GLYPHS.len() / 2];
            }
            let frac = (v - min) / span;
            let idx = (frac * (GLYPHS.len() - 1) as f64 + 0.5).floor() as usize;
            let clamped = idx.min(GLYPHS.len() - 1);
            GLYPHS[clamped]
        })
        .collect();
    if out.chars().count() < width {
        let pad = width - out.chars().count();
        let mut prefix = String::new();
        for _ in 0..pad {
            prefix.push(BLANK_GLYPH);
        }
        prefix.push_str(&out);
        out = prefix;
    }
    out
}
