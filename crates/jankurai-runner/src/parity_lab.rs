//! Target-switched parity report schema and checker.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use crate::hashing::sha256_hex;

/// One target-switched parity case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParityCase {
    /// Stable case id.
    pub id: String,
    /// Case tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Adapter kind.
    pub target_kind: String,
    /// Case steps.
    #[serde(default)]
    pub steps: Vec<ParityStep>,
    /// Performance budget.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub perf: Option<ParityPerfBudget>,
}

impl ParityCase {
    /// Whether this case is required by the approval gate.
    pub fn is_required(&self) -> bool {
        self.tags
            .iter()
            .any(|tag| tag == "required" || tag == "approved")
    }

    /// Whether this case requires performance data.
    pub fn requires_perf(&self) -> bool {
        self.perf.is_some() || self.tags.iter().any(|tag| tag == "perf")
    }
}

/// One protocol or command step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParityStep {
    /// Input to send.
    pub send: String,
    /// Expected output.
    pub expect: String,
}

/// Performance budget for a case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParityPerfBudget {
    /// Maximum candidate/reference p95 ratio.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p95_ms_max_ratio: Option<f64>,
}

/// One parity case result.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ParityResult {
    /// Case id.
    pub case_id: String,
    /// Target name, such as `reference` or `candidate`.
    pub target: String,
    /// Result status.
    pub status: String,
    /// Whether the case was skipped.
    #[serde(default)]
    pub skipped: bool,
    /// Optional message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Performance data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub perf: Option<serde_json::Value>,
    /// SHA-256 of stdout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout_sha256: Option<String>,
    /// SHA-256 of stderr.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr_sha256: Option<String>,
    /// Process exit code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Elapsed nanoseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elapsed_nanos: Option<u128>,
    /// Candidate/reference latency ratio for this case, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency_ratio: Option<f64>,
    /// Case artifact directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_dir: Option<PathBuf>,
    /// Extra diagnostics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<serde_json::Value>,
}

/// Full parity report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParityReport {
    /// Report schema version.
    pub schema_version: String,
    /// Reference target label.
    pub reference: String,
    /// Candidate target label.
    pub candidate: String,
    /// Results.
    #[serde(default)]
    pub results: Vec<ParityResult>,
}

/// RedlineDB-style parity artifact paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParityArtifacts {
    /// Generated case manifest.
    pub generated_manifest_json: PathBuf,
    /// Approved CI case id list.
    pub approved_ci_txt: PathBuf,
    /// Raw JSONL results.
    pub raw_jsonl: PathBuf,
    /// Summary JSON report.
    pub summary_json: PathBuf,
    /// Gap JSON report.
    pub gaps_json: PathBuf,
}

/// Summary written to `target/zyal/parity/<run_id>/summary.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParitySummary {
    /// Report schema version.
    pub schema_version: String,
    /// Overall status.
    pub status: String,
    /// Number of declared cases.
    pub case_count: usize,
    /// Passed result count.
    pub passed: usize,
    /// Failed result count.
    pub failed: usize,
    /// Skipped result count.
    pub skipped: usize,
    /// Required perf cases missing perf data.
    pub missing_perf: usize,
    /// Perf-budget failures.
    pub perf_over_budget: usize,
    /// Generated parity gaps.
    #[serde(default)]
    pub gaps: Vec<ParityGap>,
    /// Full report.
    pub report: ParityReport,
}

/// Redline-style generated manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedParityManifest {
    /// Manifest schema.
    pub schema_version: String,
    /// Run id.
    pub run_id: String,
    /// Number of generated cases.
    pub case_count: usize,
    /// Generated cases.
    pub cases: Vec<GeneratedParityCase>,
}

/// One generated parity case manifest row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedParityCase {
    /// Case id.
    pub id: String,
    /// Target kind.
    pub target_kind: String,
    /// Tags.
    pub tags: Vec<String>,
    /// Whether the case is approved for CI gates.
    pub approved: bool,
    /// Step count.
    pub step_count: usize,
    /// Whether performance data is required.
    pub requires_perf: bool,
}

/// Redline-style raw row written to `raw.jsonl`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawParityRow {
    /// Schema version.
    pub schema_version: String,
    /// Case id.
    pub case_id: String,
    /// Target name.
    pub target: String,
    /// Status.
    pub status: String,
    /// Whether skipped.
    pub skipped: bool,
    /// Exit code.
    pub exit_code: Option<i32>,
    /// Elapsed nanoseconds.
    pub elapsed_nanos: Option<u128>,
    /// Stdout hash.
    pub stdout_sha256: Option<String>,
    /// Stderr hash.
    pub stderr_sha256: Option<String>,
    /// Perf payload.
    pub perf: Option<serde_json::Value>,
    /// Message.
    pub message: Option<String>,
}

/// Follow-up work generated from a parity failure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParityGap {
    /// Gap id.
    pub id: String,
    /// Case id.
    pub case_id: String,
    /// Gap category.
    pub category: String,
    /// Profile or target lane.
    pub profile: String,
    /// Priority.
    pub priority: u8,
    /// Human-readable message.
    pub message: String,
    /// Follow-up task payload.
    pub follow_up_task: serde_json::Value,
}

/// Machine-check a parity report.
pub fn check_report(cases: &[ParityCase], report: &ParityReport) -> Result<()> {
    let mut errors = Vec::new();
    if cases.is_empty() {
        errors.push("zero parity cases declared".to_string());
    }
    if report.results.is_empty() {
        errors.push("zero parity results reported".to_string());
    }
    for case in cases {
        let matching: Vec<&ParityResult> = report
            .results
            .iter()
            .filter(|result| result.case_id == case.id)
            .collect();
        if case.is_required() && matching.is_empty() {
            errors.push(format!("required case {} missing from report", case.id));
            continue;
        }
        for result in matching {
            if case.is_required() && result.skipped {
                errors.push(format!("required case {} was skipped", case.id));
            }
            if result.status != "passed" {
                errors.push(format!(
                    "case {} failed with status {}",
                    case.id, result.status
                ));
            }
            if case.requires_perf() && result.perf.is_none() {
                errors.push(format!("perf case {} is missing perf data", case.id));
            }
            if let Some(budget) = case.perf.as_ref().and_then(|perf| perf.p95_ms_max_ratio) {
                if perf_ratio_for_result(result).is_some_and(|ratio| ratio > budget) {
                    errors.push(format!(
                        "perf case {} ratio exceeded budget {}",
                        case.id, budget
                    ));
                }
            }
        }
        if let Some(budget) = case.perf.as_ref().and_then(|perf| perf.p95_ms_max_ratio) {
            if candidate_reference_ratio(case, report).is_some_and(|ratio| ratio > budget) {
                errors.push(format!(
                    "perf case {} candidate/reference ratio exceeded budget {}",
                    case.id, budget
                ));
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(anyhow!(errors.join("; ")))
    }
}

/// Load parity cases from a directory of `.json` or `.toml` case files.
pub fn load_cases_from_dir(dir: &Path, approved_only: bool) -> Result<Vec<ParityCase>> {
    let mut cases = Vec::new();
    if !dir.exists() {
        return Ok(cases);
    }
    let mut entries = fs::read_dir(dir)
        .with_context(|| format!("read parity case dir {}", dir.display()))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|ext| ext.to_str());
        let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let case = match ext {
            Some("json") => serde_json::from_str::<ParityCase>(&text)
                .with_context(|| format!("parse {}", path.display()))?,
            Some("toml") => toml::from_str::<ParityCase>(&text)
                .with_context(|| format!("parse {}", path.display()))?,
            _ => continue,
        };
        if !approved_only || case.is_required() {
            cases.push(case);
        }
    }
    Ok(cases)
}

/// Return approved/required cases.
pub fn approved_cases(cases: &[ParityCase]) -> Vec<ParityCase> {
    cases
        .iter()
        .filter(|case| case.is_required())
        .cloned()
        .collect()
}

/// Target adapter for reference/candidate switched execution.
pub trait TargetAdapter {
    /// Adapter name.
    fn name(&self) -> &str;
    /// Setup before cases run.
    fn setup(&mut self) -> Result<()>;
    /// Run one case.
    fn run_case(&mut self, case: &ParityCase) -> Result<ParityResult>;
}

/// Shell command adapter. Each parity step is sent to the command stdin and
/// stdout is compared against the expected text.
#[derive(Debug, Clone)]
pub struct CommandTargetAdapter {
    name: String,
    command: String,
    cwd: PathBuf,
}

impl CommandTargetAdapter {
    /// Construct a command adapter.
    pub fn new(
        name: impl Into<String>,
        command: impl Into<String>,
        cwd: impl Into<PathBuf>,
    ) -> Self {
        Self {
            name: name.into(),
            command: command.into(),
            cwd: cwd.into(),
        }
    }
}

impl TargetAdapter for CommandTargetAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn setup(&mut self) -> Result<()> {
        Ok(())
    }

    fn run_case(&mut self, case: &ParityCase) -> Result<ParityResult> {
        let started = Instant::now();
        let mut last_stdout = Vec::new();
        let mut last_stderr = Vec::new();
        let mut last_exit_code = Some(0);
        for step in &case.steps {
            let mut child = Command::new("sh")
                .arg("-c")
                .arg(&self.command)
                .current_dir(&self.cwd)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .with_context(|| format!("spawn parity command `{}`", self.command))?;
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(step.send.as_bytes())?;
            }
            let output = child.wait_with_output()?;
            last_stdout = output.stdout.clone();
            last_stderr = output.stderr.clone();
            last_exit_code = output.status.code();
            let stdout = String::from_utf8_lossy(&output.stdout);
            if !output.status.success() {
                let elapsed = started.elapsed();
                return Ok(ParityResult {
                    case_id: case.id.clone(),
                    target: self.name.clone(),
                    status: "failed".to_string(),
                    skipped: false,
                    message: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                    perf: perf_payload(case, elapsed.as_millis() as u64, elapsed.as_nanos()),
                    stdout_sha256: Some(sha256_hex(&output.stdout)),
                    stderr_sha256: Some(sha256_hex(&output.stderr)),
                    exit_code: output.status.code(),
                    elapsed_nanos: Some(elapsed.as_nanos()),
                    latency_ratio: None,
                    artifact_dir: None,
                    diagnostics: Some(serde_json::json!({"reason": "nonzero_exit"})),
                });
            }
            if stdout.trim_end() != step.expect.trim_end() {
                let elapsed = started.elapsed();
                return Ok(ParityResult {
                    case_id: case.id.clone(),
                    target: self.name.clone(),
                    status: "failed".to_string(),
                    skipped: false,
                    message: Some(format!("expected {:?}, got {:?}", step.expect, stdout)),
                    perf: perf_payload(case, elapsed.as_millis() as u64, elapsed.as_nanos()),
                    stdout_sha256: Some(sha256_hex(&output.stdout)),
                    stderr_sha256: Some(sha256_hex(&output.stderr)),
                    exit_code: output.status.code(),
                    elapsed_nanos: Some(elapsed.as_nanos()),
                    latency_ratio: None,
                    artifact_dir: None,
                    diagnostics: Some(serde_json::json!({"reason": "stdout_mismatch"})),
                });
            }
        }
        let elapsed = started.elapsed();
        Ok(ParityResult {
            case_id: case.id.clone(),
            target: self.name.clone(),
            status: "passed".to_string(),
            skipped: false,
            message: None,
            perf: perf_payload(case, elapsed.as_millis() as u64, elapsed.as_nanos()),
            stdout_sha256: Some(sha256_hex(&last_stdout)),
            stderr_sha256: Some(sha256_hex(&last_stderr)),
            exit_code: last_exit_code,
            elapsed_nanos: Some(elapsed.as_nanos()),
            latency_ratio: None,
            artifact_dir: None,
            diagnostics: None,
        })
    }
}

/// Tiny fake adapter for deterministic smoke tests.
#[derive(Debug, Default)]
pub struct FakeTargetAdapter {
    name: String,
    fail_case_id: Option<String>,
}

impl FakeTargetAdapter {
    /// Construct a fake adapter.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fail_case_id: None,
        }
    }

    /// Configure one case id to fail.
    pub fn fail_case(mut self, case_id: impl Into<String>) -> Self {
        self.fail_case_id = Some(case_id.into());
        self
    }
}

impl TargetAdapter for FakeTargetAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn setup(&mut self) -> Result<()> {
        Ok(())
    }

    fn run_case(&mut self, case: &ParityCase) -> Result<ParityResult> {
        let failed = self.fail_case_id.as_deref() == Some(case.id.as_str());
        Ok(ParityResult {
            case_id: case.id.clone(),
            target: self.name.clone(),
            status: if failed { "failed" } else { "passed" }.to_string(),
            skipped: false,
            message: if failed {
                Some("fake failure".into())
            } else {
                None
            },
            perf: case
                .requires_perf()
                .then(|| serde_json::json!({"p95_ms": 1.0, "elapsed_nanos": 1_000_000})),
            stdout_sha256: Some(sha256_hex(b"fake")),
            stderr_sha256: Some(sha256_hex(b"")),
            exit_code: Some(if failed { 1 } else { 0 }),
            elapsed_nanos: Some(1_000_000),
            latency_ratio: None,
            artifact_dir: None,
            diagnostics: None,
        })
    }
}

/// Run a case list against an adapter and return a report.
pub fn run_cases<A: TargetAdapter>(
    adapter: &mut A,
    cases: &[ParityCase],
    reference: &str,
    candidate: &str,
) -> Result<ParityReport> {
    adapter.setup()?;
    let mut results = Vec::new();
    for case in cases {
        results.push(adapter.run_case(case)?);
    }
    Ok(ParityReport {
        schema_version: "zyal.parity.v1".to_string(),
        reference: reference.to_string(),
        candidate: candidate.to_string(),
        results,
    })
}

/// Run cases against reference and candidate adapters.
pub fn run_target_switched_cases<A: TargetAdapter, B: TargetAdapter>(
    reference_adapter: &mut A,
    candidate_adapter: &mut B,
    cases: &[ParityCase],
) -> Result<ParityReport> {
    reference_adapter.setup()?;
    candidate_adapter.setup()?;
    let mut results = Vec::new();
    for case in cases {
        results.push(reference_adapter.run_case(case)?);
        results.push(candidate_adapter.run_case(case)?);
    }
    Ok(ParityReport {
        schema_version: "zyal.parity.v1".to_string(),
        reference: reference_adapter.name().to_string(),
        candidate: candidate_adapter.name().to_string(),
        results,
    })
}

/// Write RedlineDB-style raw JSONL and summary JSON artifacts.
pub fn write_report_artifacts(
    repo_root: &Path,
    run_id: &str,
    cases: &[ParityCase],
    report: ParityReport,
) -> Result<ParityArtifacts> {
    let dir = repo_root.join("target/zyal/parity").join(run_id);
    fs::create_dir_all(&dir).with_context(|| format!("mkdir {}", dir.display()))?;
    let generated_manifest_json = dir.join("generated_manifest.json");
    let approved_ci_txt = dir.join("approved-ci.txt");
    let raw_jsonl = dir.join("raw.jsonl");
    let summary_json = dir.join("summary.json");
    let gaps_json = dir.join("gaps.json");
    let mut raw = fs::File::create(&raw_jsonl)?;
    for result in &report.results {
        let row = RawParityRow {
            schema_version: "zyal.parity.raw.v1".to_string(),
            case_id: result.case_id.clone(),
            target: result.target.clone(),
            status: result.status.clone(),
            skipped: result.skipped,
            exit_code: result.exit_code,
            elapsed_nanos: result.elapsed_nanos,
            stdout_sha256: result.stdout_sha256.clone(),
            stderr_sha256: result.stderr_sha256.clone(),
            perf: result.perf.clone(),
            message: result.message.clone(),
        };
        writeln!(raw, "{}", serde_json::to_string(&row)?)?;
    }
    let summary = summarize_report(cases, report);
    let manifest = generated_manifest(run_id, cases);
    fs::write(
        &generated_manifest_json,
        serde_json::to_string_pretty(&manifest)?,
    )?;
    fs::write(&approved_ci_txt, approved_ci_text(cases))?;
    fs::write(&summary_json, serde_json::to_string_pretty(&summary)?)?;
    fs::write(&gaps_json, serde_json::to_string_pretty(&summary.gaps)?)?;
    Ok(ParityArtifacts {
        generated_manifest_json,
        approved_ci_txt,
        raw_jsonl,
        summary_json,
        gaps_json,
    })
}

/// Build a Redline-style generated manifest.
pub fn generated_manifest(run_id: &str, cases: &[ParityCase]) -> GeneratedParityManifest {
    GeneratedParityManifest {
        schema_version: "zyal.parity.generated_manifest.v1".to_string(),
        run_id: run_id.to_string(),
        case_count: cases.len(),
        cases: cases
            .iter()
            .map(|case| GeneratedParityCase {
                id: case.id.clone(),
                target_kind: case.target_kind.clone(),
                tags: case.tags.clone(),
                approved: case.is_required(),
                step_count: case.steps.len(),
                requires_perf: case.requires_perf(),
            })
            .collect(),
    }
}

fn approved_ci_text(cases: &[ParityCase]) -> String {
    let mut ids: Vec<&str> = cases
        .iter()
        .filter(|case| case.is_required())
        .map(|case| case.id.as_str())
        .collect();
    ids.sort_unstable();
    if ids.is_empty() {
        String::new()
    } else {
        format!("{}\n", ids.join("\n"))
    }
}

/// Check a summary artifact as the parity check lane.
pub fn check_summary_artifact(path: &Path, cases: &[ParityCase]) -> Result<()> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let summary: ParitySummary = serde_json::from_str(&text).context("parse parity summary")?;
    check_report(cases, &summary.report)
}

/// Build a summary payload.
pub fn summarize_report(cases: &[ParityCase], report: ParityReport) -> ParitySummary {
    let passed = report
        .results
        .iter()
        .filter(|result| result.status == "passed" && !result.skipped)
        .count();
    let failed = report
        .results
        .iter()
        .filter(|result| result.status != "passed")
        .count();
    let skipped = report
        .results
        .iter()
        .filter(|result| result.skipped)
        .count();
    let missing_perf = cases
        .iter()
        .filter(|case| case.requires_perf())
        .filter(|case| {
            report
                .results
                .iter()
                .filter(|result| result.case_id == case.id)
                .any(|result| result.perf.is_none())
        })
        .count();
    let gaps = generate_gaps(cases, &report);
    let perf_over_budget = gaps
        .iter()
        .filter(|gap| gap.category == "perf_budget")
        .count();
    let status = if check_report(cases, &report).is_ok() {
        "passed"
    } else {
        "failed"
    };
    ParitySummary {
        schema_version: "zyal.parity.summary.v1".to_string(),
        status: status.to_string(),
        case_count: cases.len(),
        passed,
        failed,
        skipped,
        missing_perf,
        perf_over_budget,
        gaps,
        report,
    }
}

/// Generate follow-up gaps from a report.
pub fn generate_gaps(cases: &[ParityCase], report: &ParityReport) -> Vec<ParityGap> {
    let mut gaps = Vec::new();
    if cases.is_empty() {
        gaps.push(gap(
            "zero-cases",
            "suite",
            "missing_case",
            "parity",
            1,
            "zero parity cases declared",
        ));
        return gaps;
    }
    for case in cases {
        let matching: Vec<&ParityResult> = report
            .results
            .iter()
            .filter(|result| result.case_id == case.id)
            .collect();
        if case.is_required() && matching.is_empty() {
            gaps.push(gap(
                &format!("missing-{}", case.id),
                &case.id,
                "missing_required",
                "correctness",
                1,
                "required parity case missing from report",
            ));
            continue;
        }
        for result in matching {
            if case.is_required() && result.skipped {
                gaps.push(gap(
                    &format!("skipped-{}-{}", case.id, result.target),
                    &case.id,
                    "skipped_required",
                    "correctness",
                    1,
                    "required parity case was skipped",
                ));
            }
            if result.status != "passed" {
                gaps.push(gap(
                    &format!("failed-{}-{}", case.id, result.target),
                    &case.id,
                    "failed_case",
                    "correctness",
                    1,
                    result.message.as_deref().unwrap_or("case failed"),
                ));
            }
            if case.requires_perf() && result.perf.is_none() {
                gaps.push(gap(
                    &format!("missing-perf-{}-{}", case.id, result.target),
                    &case.id,
                    "missing_perf",
                    "performance",
                    2,
                    "required perf case is missing perf data",
                ));
            }
            if let Some(budget) = case.perf.as_ref().and_then(|perf| perf.p95_ms_max_ratio) {
                if perf_ratio_for_result(result).is_some_and(|ratio| ratio > budget) {
                    gaps.push(gap(
                        &format!("perf-{}-{}", case.id, result.target),
                        &case.id,
                        "perf_budget",
                        "performance",
                        1,
                        &format!("latency ratio exceeded budget {budget}"),
                    ));
                }
            }
        }
        if let Some(budget) = case.perf.as_ref().and_then(|perf| perf.p95_ms_max_ratio) {
            if candidate_reference_ratio(case, report).is_some_and(|ratio| ratio > budget) {
                gaps.push(gap(
                    &format!("perf-ratio-{}", case.id),
                    &case.id,
                    "perf_budget",
                    "performance",
                    1,
                    &format!("candidate/reference p95 ratio exceeded budget {budget}"),
                ));
            }
        }
    }
    gaps
}

fn gap(
    id: &str,
    case_id: &str,
    category: &str,
    profile: &str,
    priority: u8,
    message: &str,
) -> ParityGap {
    ParityGap {
        id: id.to_string(),
        case_id: case_id.to_string(),
        category: category.to_string(),
        profile: profile.to_string(),
        priority,
        message: message.to_string(),
        follow_up_task: serde_json::json!({
            "title": format!("Close parity gap {case_id}: {category}"),
            "category": category,
            "profile": profile,
            "priority": priority,
        }),
    }
}

fn perf_payload(
    case: &ParityCase,
    elapsed_ms: u64,
    elapsed_nanos: u128,
) -> Option<serde_json::Value> {
    case.requires_perf().then(|| {
        serde_json::json!({
            "duration_ms": elapsed_ms,
            "p95_ms": elapsed_ms.max(1),
            "elapsed_nanos": elapsed_nanos,
            "captured_at": now_secs(),
        })
    })
}

fn perf_ratio_for_result(result: &ParityResult) -> Option<f64> {
    result
        .latency_ratio
        .or_else(|| result.perf.as_ref()?.get("latency_ratio")?.as_f64())
        .or_else(|| result.perf.as_ref()?.get("p95_ms_ratio")?.as_f64())
}

fn candidate_reference_ratio(case: &ParityCase, report: &ParityReport) -> Option<f64> {
    let reference = report
        .results
        .iter()
        .find(|result| result.case_id == case.id && result.target == report.reference)
        .and_then(p95_ms)?;
    let candidate = report
        .results
        .iter()
        .find(|result| result.case_id == case.id && result.target == report.candidate)
        .and_then(p95_ms)?;
    if reference <= 0.0 {
        None
    } else {
        Some(candidate / reference)
    }
}

fn p95_ms(result: &ParityResult) -> Option<f64> {
    result.perf.as_ref()?.get("p95_ms")?.as_f64()
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn required_case(id: &str) -> ParityCase {
        ParityCase {
            id: id.into(),
            tags: vec!["required".into(), "approved".into()],
            target_kind: "fake".into(),
            steps: vec![ParityStep {
                send: "PING".into(),
                expect: "PONG".into(),
            }],
            perf: None,
        }
    }

    #[test]
    fn checker_accepts_required_pass() {
        let cases = vec![required_case("ping")];
        let report = ParityReport {
            schema_version: "zyal.parity.v1".into(),
            reference: "ref".into(),
            candidate: "cand".into(),
            results: vec![ParityResult {
                case_id: "ping".into(),
                target: "cand".into(),
                status: "passed".into(),
                skipped: false,
                message: None,
                perf: None,
                ..ParityResult::default()
            }],
        };
        check_report(&cases, &report).unwrap();
    }

    #[test]
    fn checker_rejects_missing_skipped_failed_and_perfless_cases() {
        let mut perf_case = required_case("perf");
        perf_case.perf = Some(ParityPerfBudget {
            p95_ms_max_ratio: Some(1.25),
        });
        let cases = vec![
            required_case("missing"),
            required_case("skip"),
            required_case("fail"),
            perf_case,
        ];
        let report = ParityReport {
            schema_version: "zyal.parity.v1".into(),
            reference: "ref".into(),
            candidate: "cand".into(),
            results: vec![
                ParityResult {
                    case_id: "skip".into(),
                    target: "cand".into(),
                    status: "passed".into(),
                    skipped: true,
                    message: None,
                    perf: None,
                    ..ParityResult::default()
                },
                ParityResult {
                    case_id: "fail".into(),
                    target: "cand".into(),
                    status: "failed".into(),
                    skipped: false,
                    message: Some("no".into()),
                    perf: None,
                    ..ParityResult::default()
                },
                ParityResult {
                    case_id: "perf".into(),
                    target: "cand".into(),
                    status: "passed".into(),
                    skipped: false,
                    message: None,
                    perf: None,
                    ..ParityResult::default()
                },
            ],
        };
        let err = check_report(&cases, &report).unwrap_err().to_string();
        assert!(err.contains("missing"));
        assert!(err.contains("skipped"));
        assert!(err.contains("failed"));
        assert!(err.contains("missing perf"));
    }

    #[test]
    fn fake_adapter_produces_checkable_report() {
        let cases = vec![required_case("ping")];
        let mut adapter = FakeTargetAdapter::new("candidate");
        let report = run_cases(&mut adapter, &cases, "reference", "candidate").unwrap();
        check_report(&cases, &report).unwrap();
    }

    #[test]
    fn command_adapter_passes_and_fails_cases() {
        let mut pass = required_case("echo-pass");
        pass.steps[0].expect = "PING".into();
        let mut adapter = CommandTargetAdapter::new("candidate", "cat", ".");
        let result = adapter.run_case(&pass).unwrap();
        assert_eq!(result.status, "passed");

        let mut fail = required_case("echo-fail");
        fail.steps[0].expect = "NOPE".into();
        let result = adapter.run_case(&fail).unwrap();
        assert_eq!(result.status, "failed");
    }

    #[test]
    fn writes_raw_and_summary_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let cases = vec![required_case("ping")];
        let report = ParityReport {
            schema_version: "zyal.parity.v1".into(),
            reference: "ref".into(),
            candidate: "cand".into(),
            results: vec![ParityResult {
                case_id: "ping".into(),
                target: "cand".into(),
                status: "passed".into(),
                skipped: false,
                message: None,
                perf: None,
                ..ParityResult::default()
            }],
        };
        let artifacts = write_report_artifacts(dir.path(), "run-1", &cases, report).unwrap();
        assert!(artifacts.generated_manifest_json.exists());
        assert!(artifacts.approved_ci_txt.exists());
        assert!(artifacts.raw_jsonl.exists());
        assert!(artifacts.summary_json.exists());
        assert!(artifacts.gaps_json.exists());
        let approved = fs::read_to_string(artifacts.approved_ci_txt).unwrap();
        assert_eq!(approved.trim(), "ping");
        check_summary_artifact(&artifacts.summary_json, &cases).unwrap();
    }

    #[test]
    fn summary_marks_perf_missing_gate_failure() {
        let mut perf_case = required_case("perf");
        perf_case.perf = Some(ParityPerfBudget {
            p95_ms_max_ratio: Some(1.25),
        });
        let report = ParityReport {
            schema_version: "zyal.parity.v1".into(),
            reference: "ref".into(),
            candidate: "cand".into(),
            results: vec![ParityResult {
                case_id: "perf".into(),
                target: "cand".into(),
                status: "passed".into(),
                skipped: false,
                message: None,
                perf: None,
                ..ParityResult::default()
            }],
        };
        let summary = summarize_report(&[perf_case], report);
        assert_eq!(summary.status, "failed");
        assert_eq!(summary.missing_perf, 1);
    }
}
