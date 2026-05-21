use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};
use chrono::Utc;
use clap::ValueEnum;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SecurityProfile {
    Local,
    Ci,
}

impl SecurityProfile {
    fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Ci => "ci",
        }
    }

    fn requires_syft(self) -> bool {
        matches!(self, Self::Ci)
    }
}

#[derive(Serialize)]
struct SecurityEvidence {
    schema_version: &'static str,
    generated_at: String,
    profile: &'static str,
    status: &'static str,
    checks: Vec<SecurityCheck>,
}

#[derive(Serialize)]
struct SecurityCheck {
    name: String,
    tool: String,
    status: String,
    detail: String,
    artifact: Option<String>,
}

pub fn run(out: &Path, profile: SecurityProfile) -> Result<()> {
    fs::create_dir_all(out).with_context(|| format!("create {}", out.display()))?;

    let gitleaks_report = out.join("gitleaks.json");
    let cargo_audit_report = out.join("cargo-audit.json");
    let zizmor_report = out.join("zizmor.json");
    let sbom_report = out.join("sbom.spdx.json");

    let gitleaks = probe(
        "gitleaks",
        &["version"],
        "secret scan tool availability",
        true,
    );
    let cargo_audit = probe(
        "cargo",
        &["audit", "--version"],
        "Rust dependency audit tool availability",
        true,
    );
    let syft = probe(
        "syft",
        &["version"],
        "SBOM tool availability",
        profile.requires_syft(),
    );
    let zizmor = probe(
        "zizmor",
        &["--version"],
        "GitHub Actions workflow audit tool availability",
        profile.requires_syft(),
    );

    let gitleaks_available = gitleaks.is_available();
    let cargo_audit_available = cargo_audit.is_available();
    let syft_available = syft.is_available();
    let zizmor_available = zizmor.is_available();

    let mut checks = vec![gitleaks, cargo_audit, syft, zizmor];
    if gitleaks_available {
        checks.push(run_gitleaks(&gitleaks_report)?);
    }
    if cargo_audit_available {
        checks.push(run_cargo_audit(&cargo_audit_report)?);
    }
    if zizmor_available {
        checks.push(run_zizmor(&zizmor_report)?);
    }
    if syft_available {
        checks.push(run_syft(&sbom_report)?);
    }

    let status = if checks.iter().all(SecurityCheck::is_passing) {
        "ok"
    } else {
        "failed"
    };
    let evidence = SecurityEvidence {
        schema_version: "1.0.0",
        generated_at: Utc::now().to_rfc3339(),
        profile: profile.as_str(),
        status,
        checks,
    };

    let evidence_path = out.join("evidence.json");
    let body = serde_json::to_string_pretty(&evidence)?;
    fs::write(&evidence_path, format!("{body}\n"))
        .with_context(|| format!("write {}", evidence_path.display()))?;
    fs::write(out.join("lane-status.txt"), format!("{status}\n"))
        .with_context(|| format!("write {}", out.join("lane-status.txt").display()))?;
    println!("security-lane: wrote {}", evidence_path.display());
    if status != "ok" {
        bail!("security-lane failed; see {}", evidence_path.display());
    }
    Ok(())
}

impl SecurityCheck {
    fn is_available(&self) -> bool {
        self.status == "available"
    }

    fn is_passing(&self) -> bool {
        matches!(
            self.status.as_str(),
            "ok" | "available" | "advisory-missing"
        )
    }
}

fn probe(tool: &'static str, args: &[&str], detail: &'static str, required: bool) -> SecurityCheck {
    let output = Command::new(tool).args(args).output();
    let available = output
        .as_ref()
        .map(|output| output.status.success())
        .unwrap_or(false);
    SecurityCheck {
        name: detail.to_string(),
        tool: tool.to_string(),
        status: if available {
            "available".to_string()
        } else if required {
            "missing".to_string()
        } else {
            "advisory-missing".to_string()
        },
        detail: match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let text = if stdout.trim().is_empty() {
                    stderr.trim()
                } else {
                    stdout.trim()
                };
                if text.is_empty() {
                    format!("exit {}", output.status)
                } else {
                    text.lines().next().unwrap_or_default().to_string()
                }
            }
            Err(err) => err.to_string(),
        },
        artifact: None,
    }
}

fn run_gitleaks(report: &Path) -> Result<SecurityCheck> {
    if let Some(parent) = report.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let output = Command::new("gitleaks")
        .args([
            "detect",
            "--source",
            ".",
            "--no-git",
            "--redact",
            "--report-format",
            "json",
            "--report-path",
        ])
        .arg(report)
        .output()
        .context("run gitleaks detect")?;
    if output.status.success() && !report.exists() {
        fs::write(report, "[]\n").with_context(|| format!("write {}", report.display()))?;
    }
    Ok(command_check(
        "secret scan",
        "gitleaks",
        output,
        Some(report.display().to_string()),
    ))
}

fn run_cargo_audit(report: &Path) -> Result<SecurityCheck> {
    if let Some(parent) = report.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let output = Command::new("cargo")
        .args(["audit", "--json"])
        .output()
        .context("run cargo audit --json")?;
    let body = if output.stdout.is_empty() {
        &output.stderr
    } else {
        &output.stdout
    };
    fs::write(report, body).with_context(|| format!("write {}", report.display()))?;
    Ok(command_check(
        "Rust dependency audit",
        "cargo audit",
        output,
        Some(report.display().to_string()),
    ))
}

fn run_zizmor(report: &Path) -> Result<SecurityCheck> {
    if let Some(parent) = report.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let output = Command::new("zizmor")
        .args([
            "--offline",
            "--no-exit-codes",
            "--format",
            "json",
            ".github/workflows",
        ])
        .output()
        .context("run zizmor workflow audit")?;
    let body = if output.stdout.is_empty() {
        &output.stderr
    } else {
        &output.stdout
    };
    fs::write(report, body).with_context(|| format!("write {}", report.display()))?;
    Ok(command_check(
        "GitHub Actions workflow audit",
        "zizmor",
        output,
        Some(report.display().to_string()),
    ))
}

fn run_syft(report: &Path) -> Result<SecurityCheck> {
    if let Some(parent) = report.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let format = format!("spdx-json={}", report.display());
    let output = Command::new("syft")
        .args([".", "-o", &format])
        .output()
        .context("run syft SBOM generation")?;
    Ok(command_check(
        "SBOM generation",
        "syft",
        output,
        Some(report.display().to_string()),
    ))
}

fn command_check(
    name: &str,
    tool: &str,
    output: std::process::Output,
    artifact: Option<String>,
) -> SecurityCheck {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let text = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };
    SecurityCheck {
        name: name.to_string(),
        tool: tool.to_string(),
        status: if output.status.success() {
            "ok".to_string()
        } else {
            "failed".to_string()
        },
        detail: if text.is_empty() {
            format!("exit {}", output.status)
        } else {
            text.lines().next().unwrap_or_default().to_string()
        },
        artifact,
    }
}
