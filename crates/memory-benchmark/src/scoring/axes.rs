#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AxisScores {
    pub correctness: f32,
    pub provenance: f32,
    pub bitemporal_recall: f32,
    pub contradiction: f32,
    pub math_science: f32,
    pub english_discourse_coreference: f32,
    pub privacy_redaction: f32,
    pub procedural_skill: f32,
    pub feedback_adaptation: f32,
    pub determinism_rebuild: f32,
}

impl AxisScores {
    pub const WEIGHTS: AxisScores = AxisScores {
        correctness: 20.0,
        provenance: 12.0,
        bitemporal_recall: 10.0,
        contradiction: 10.0,
        math_science: 12.0,
        english_discourse_coreference: 8.0,
        privacy_redaction: 8.0,
        procedural_skill: 8.0,
        feedback_adaptation: 6.0,
        determinism_rebuild: 6.0,
    };

    pub const ADVANCED_WEIGHTS: [(&'static str, f32); 12] = [
        ("concept_learning", 12.0),
        ("transfer_reasoning", 12.0),
        ("formal_math", 10.0),
        ("scientific_reasoning", 10.0),
        ("bitemporal_correctness", 10.0),
        ("provenance_support", 10.0),
        ("dependency_maintenance", 8.0),
        ("contradiction_skepticism", 8.0),
        ("privacy_forgetting", 8.0),
        ("compression_fidelity", 5.0),
        ("procedural_tool_memory", 4.0),
        ("determinism_efficiency", 3.0),
    ];

    pub fn weighted(&self) -> f32 {
        let w = Self::WEIGHTS;
        self.correctness * w.correctness
            + self.provenance * w.provenance
            + self.bitemporal_recall * w.bitemporal_recall
            + self.contradiction * w.contradiction
            + self.math_science * w.math_science
            + self.english_discourse_coreference * w.english_discourse_coreference
            + self.privacy_redaction * w.privacy_redaction
            + self.procedural_skill * w.procedural_skill
            + self.feedback_adaptation * w.feedback_adaptation
            + self.determinism_rebuild * w.determinism_rebuild
    }

    pub fn from_single(axis: ScoringAxis, value: f32) -> Self {
        let mut s = Self::default();
        match axis {
            ScoringAxis::Correctness => s.correctness = value,
            ScoringAxis::Provenance => s.provenance = value,
            ScoringAxis::BitemporalRecall => s.bitemporal_recall = value,
            ScoringAxis::Contradiction => s.contradiction = value,
            ScoringAxis::MathScience => s.math_science = value,
            ScoringAxis::EnglishDiscourseCoreference => s.english_discourse_coreference = value,
            ScoringAxis::PrivacyRedaction => s.privacy_redaction = value,
            ScoringAxis::ProceduralSkill => s.procedural_skill = value,
            ScoringAxis::FeedbackAdaptation => s.feedback_adaptation = value,
            ScoringAxis::DeterminismRebuild => s.determinism_rebuild = value,
        }
        s
    }

    pub fn merge_max(&mut self, other: Self) {
        self.correctness = self.correctness.max(other.correctness);
        self.provenance = self.provenance.max(other.provenance);
        self.bitemporal_recall = self.bitemporal_recall.max(other.bitemporal_recall);
        self.contradiction = self.contradiction.max(other.contradiction);
        self.math_science = self.math_science.max(other.math_science);
        self.english_discourse_coreference = self
            .english_discourse_coreference
            .max(other.english_discourse_coreference);
        self.privacy_redaction = self.privacy_redaction.max(other.privacy_redaction);
        self.procedural_skill = self.procedural_skill.max(other.procedural_skill);
        self.feedback_adaptation = self.feedback_adaptation.max(other.feedback_adaptation);
        self.determinism_rebuild = self.determinism_rebuild.max(other.determinism_rebuild);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoringAxis {
    Correctness,
    Provenance,
    BitemporalRecall,
    Contradiction,
    MathScience,
    EnglishDiscourseCoreference,
    PrivacyRedaction,
    ProceduralSkill,
    FeedbackAdaptation,
    DeterminismRebuild,
}

impl ScoringAxis {
    pub fn name(self) -> &'static str {
        match self {
            ScoringAxis::Correctness => "correctness",
            ScoringAxis::Provenance => "provenance",
            ScoringAxis::BitemporalRecall => "bitemporal_recall",
            ScoringAxis::Contradiction => "contradiction",
            ScoringAxis::MathScience => "math_science",
            ScoringAxis::EnglishDiscourseCoreference => "english_discourse_coreference",
            ScoringAxis::PrivacyRedaction => "privacy_redaction",
            ScoringAxis::ProceduralSkill => "procedural_skill",
            ScoringAxis::FeedbackAdaptation => "feedback_adaptation",
            ScoringAxis::DeterminismRebuild => "determinism_rebuild",
        }
    }
}
