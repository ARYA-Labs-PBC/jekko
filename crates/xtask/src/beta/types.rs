use serde::Deserialize;

pub(super) const MODEL: &str = "jekko/gpt-5.3-codex";

#[derive(Clone, Debug, Deserialize)]
pub(super) struct Pr {
    pub number: u64,
    pub title: String,
}

#[derive(Debug)]
pub(super) struct FailedPr {
    pub number: u64,
    pub title: String,
    pub reason: String,
}

pub(super) fn lines(prs: Vec<Pr>) -> String {
    prs.into_iter()
        .map(|x| format!("- #{}: {}", x.number, x.title))
        .collect::<Vec<_>>()
        .join("\n")
}
