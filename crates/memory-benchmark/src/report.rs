//! Report writer — JSON + Markdown.
//!
//! Converts the benchmark score into deterministic JSON and Markdown payloads.

use crate::json::{self, Json};
use crate::memory_api::axes_to_json;
use crate::AxisScores;

pub struct CandidateScore {
    pub name: String,
    pub total: f32,
    pub axes: AxisScores,
    pub fixtures_run: u32,
    pub fixtures_passed: u32,
}

impl CandidateScore {
    pub fn to_json(&self) -> Json {
        json::obj(&[
            ("name", Json::Str(self.name.clone())),
            ("total", Json::Float(self.total as f64)),
            ("axes", axes_to_json(&self.axes)),
            ("fixtures_run", Json::Int(self.fixtures_run as i64)),
            ("fixtures_passed", Json::Int(self.fixtures_passed as i64)),
        ])
    }
}
