//! Loader and scorer for native real-paper QBank challenge records.
//!
//! Candidate systems observe only publication sections and context distractors.
//! Answer keys are parsed solely for post-recall grading.

mod model;
mod parse;
mod run;
mod validation;

pub use model::{
    stable_challenge_hash, stable_section_hash, AnswerKey, BankValidation, ContextPack,
    LoadedChallenge, NumericTolerance, PaperChallenge, PaperRecord, PaperSection, SupportRef,
};
pub use run::{default_bank_path, load_bank, load_challenges, run_candidate};
pub use validation::validate_bank;

#[cfg(test)]
mod tests;
