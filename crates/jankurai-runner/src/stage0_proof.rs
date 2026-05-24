//! Stage-0 proof and deterministic parity seed helpers.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde_json::json;

use crate::evidence::LoadedEvidence;
use crate::parity_lab::{ParityCase, ParityPerfBudget, ParityStep};
use crate::port::{
    MasterTaskStatus, PhaseStatus, PortMasterPlan, PortMasterTask, PortStage, PortTargetRequest,
};

pub(crate) fn evidence_prompt_fragment(evidence: &[LoadedEvidence]) -> String {
    if evidence.is_empty() {
        return "no external evidence inputs configured".to_string();
    }
    evidence
        .iter()
        .map(|item| {
            let excerpt = item.content.chars().take(1200).collect::<String>();
            format!(
                "[{} role={} source={} bytes={} clipped={}]\n{}",
                item.id, item.role, item.source, item.bytes_read, item.clipped, excerpt
            )
        })
        .collect::<Vec<_>>()
        .join("\n---\n")
}

pub(crate) fn build_stage0_master_plan(
    target: PortTargetRequest,
    evidence: &[LoadedEvidence],
) -> PortMasterPlan {
    let topics = evidence_topics(evidence, 8);
    let topics = if topics.is_empty() {
        vec![
            "contract".to_string(),
            "behavior".to_string(),
            "parity".to_string(),
        ]
    } else {
        topics
    };
    let stages: Vec<PortStage> = topics
        .iter()
        .enumerate()
        .map(|(idx, topic)| PortStage {
            id: format!("stage-{:02}-{}", idx + 1, slug(topic)),
            ordinal: idx + 1,
            name: format!("{topic} evidence"),
            objective: format!(
                "Derive, implement, and verify target behavior for evidence topic `{topic}`."
            ),
            status: if idx == 0 {
                PhaseStatus::Drafting
            } else {
                PhaseStatus::Planned
            },
        })
        .collect();
    let tasks = stages
        .iter()
        .map(|stage| PortMasterTask {
            id: format!("task-{}", stage.id.trim_start_matches("stage-")),
            stage_id: stage.id.clone(),
            title: format!(
                "{}: close target-derived behavior for {}",
                target.replacement, stage.name
            ),
            write_scope: vec!["src/**".to_string(), "tests/**".to_string()],
            proof_lane: "rtk just zyal-port-fast".to_string(),
            status: MasterTaskStatus::Queued,
        })
        .collect();
    PortMasterPlan {
        target,
        stages,
        tasks,
    }
}

pub(crate) fn parse_model_master_plan(
    target: PortTargetRequest,
    value: &serde_json::Value,
) -> Result<PortMasterPlan> {
    let plan_value = value.get("plan").unwrap_or(value);
    if let Ok(mut plan) = serde_json::from_value::<PortMasterPlan>(plan_value.clone()) {
        plan.target = target;
        validate_master_plan(&plan)?;
        return Ok(plan);
    }
    let stages_value = plan_value
        .get("stages")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow!("missing stages array"))?;
    let mut stages = Vec::new();
    for (idx, stage) in stages_value.iter().enumerate() {
        let name =
            string_field(stage, &["name", "title"]).unwrap_or_else(|| format!("stage {}", idx + 1));
        let id = string_field(stage, &["id"])
            .unwrap_or_else(|| format!("stage-{:02}-{}", idx + 1, slug(&name)));
        let objective = string_field(stage, &["objective", "summary", "description"])
            .unwrap_or_else(|| format!("Complete target-derived work for {name}."));
        stages.push(PortStage {
            id,
            ordinal: idx + 1,
            name,
            objective,
            status: if idx == 0 {
                PhaseStatus::Drafting
            } else {
                PhaseStatus::Planned
            },
        });
    }
    let mut tasks = Vec::new();
    if let Some(task_values) = plan_value
        .get("tasks")
        .and_then(serde_json::Value::as_array)
    {
        for (idx, task) in task_values.iter().enumerate() {
            let title = string_field(task, &["title", "name"])
                .unwrap_or_else(|| format!("task {}", idx + 1));
            let default_stage = stages
                .get(idx.min(stages.len().saturating_sub(1)))
                .map(|stage| stage.id.clone())
                .unwrap_or_else(|| "stage-01".to_string());
            let stage_id = string_field(task, &["stage_id", "phase_id"]).unwrap_or(default_stage);
            tasks.push(PortMasterTask {
                id: string_field(task, &["id"])
                    .unwrap_or_else(|| format!("task-{:02}-{}", idx + 1, slug(&title))),
                stage_id,
                title,
                write_scope: string_array_field(task, "write_scope")
                    .unwrap_or_else(|| vec!["src/**".to_string(), "tests/**".to_string()]),
                proof_lane: string_field(task, &["proof_lane", "proof"])
                    .unwrap_or_else(|| "rtk just zyal-port-fast".to_string()),
                status: MasterTaskStatus::Queued,
            });
        }
    }
    if tasks.is_empty() {
        tasks = stages
            .iter()
            .map(|stage| PortMasterTask {
                id: format!("task-{}", stage.id.trim_start_matches("stage-")),
                stage_id: stage.id.clone(),
                title: format!("Implement and verify {}", stage.name),
                write_scope: vec!["src/**".to_string(), "tests/**".to_string()],
                proof_lane: "rtk just zyal-port-fast".to_string(),
                status: MasterTaskStatus::Queued,
            })
            .collect();
    }
    let plan = PortMasterPlan {
        target,
        stages,
        tasks,
    };
    validate_master_plan(&plan)?;
    Ok(plan)
}

pub(crate) fn write_stage0_master_plan(
    repo: &Path,
    run_id: &str,
    plan: &PortMasterPlan,
    evidence: &[LoadedEvidence],
) -> Result<PathBuf> {
    let path = repo
        .join("target/zyal/reasoning")
        .join(run_id)
        .join("stage0-master-plan.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let payload = json!({
        "schema_version": "zyal.stage0_master_plan.v1",
        "run_id": run_id,
        "source": "runtime_evidence",
        "evidence": evidence.iter().map(LoadedEvidence::receipt).collect::<Vec<_>>(),
        "plan": plan,
    });
    fs::write(&path, serde_json::to_string_pretty(&payload)?)
        .with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

pub(crate) fn generate_seed_cases(
    target: &PortTargetRequest,
    evidence: &[LoadedEvidence],
    model_value: &serde_json::Value,
) -> Vec<ParityCase> {
    let topics = evidence_topics(evidence, 6);
    if topics.is_empty() {
        return vec![generic_smoke_case()];
    }
    let commands = command_tokens(evidence);
    let target_kind = slug(&target.target);
    topics
        .iter()
        .enumerate()
        .map(|(idx, topic)| {
            let command = commands
                .get(idx % commands.len().max(1))
                .cloned()
                .unwrap_or_else(|| topic.to_ascii_uppercase());
            ParityCase {
                id: format!("{}.{}.seed", target_kind, slug(topic)),
                tags: vec![
                    "required".to_string(),
                    "approved".to_string(),
                    "generated".to_string(),
                    "seed".to_string(),
                ],
                target_kind: target_kind.clone(),
                steps: vec![ParityStep {
                    send: command,
                    expect: "OK".to_string(),
                }],
                perf: Some(ParityPerfBudget {
                    p95_ms_max_ratio: model_value
                        .get("p95_ms_max_ratio")
                        .and_then(serde_json::Value::as_f64)
                        .or(Some(1.25)),
                }),
            }
        })
        .collect()
}

pub(crate) fn benchmark_prompt(target: &PortTargetRequest, evidence: &[LoadedEvidence]) -> String {
    format!(
        "Return JSON. Reconcile conflicting architecture evidence into a clean-room, parity-first execution plan for {} -> {}. Include evidence coverage, unsupported claims, parity cases, Jankurai/proof integration, and monitorability.\n{}",
        target.target,
        target.replacement,
        evidence_prompt_fragment(evidence)
    )
}

fn generic_smoke_case() -> ParityCase {
    ParityCase {
        id: "port.capture.request".to_string(),
        tags: vec![
            "required".to_string(),
            "approved".to_string(),
            "smoke".to_string(),
        ],
        target_kind: "generic".to_string(),
        steps: vec![ParityStep {
            send: "PING".to_string(),
            expect: "PONG".to_string(),
        }],
        perf: Some(ParityPerfBudget {
            p95_ms_max_ratio: Some(1.25),
        }),
    }
}

fn validate_master_plan(plan: &PortMasterPlan) -> Result<()> {
    if plan.stages.is_empty() {
        anyhow::bail!("master plan has no stages");
    }
    if plan.tasks.is_empty() {
        anyhow::bail!("master plan has no tasks");
    }
    let stage_ids = plan
        .stages
        .iter()
        .map(|stage| stage.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    for stage in &plan.stages {
        if stage.id.trim().is_empty() || stage.name.trim().is_empty() {
            anyhow::bail!("stage id and name are required");
        }
    }
    for task in &plan.tasks {
        if task.id.trim().is_empty() || task.title.trim().is_empty() {
            anyhow::bail!("task id and title are required");
        }
        if !stage_ids.contains(task.stage_id.as_str()) {
            anyhow::bail!(
                "task {} references unknown stage {}",
                task.id,
                task.stage_id
            );
        }
    }
    Ok(())
}

fn evidence_topics(evidence: &[LoadedEvidence], limit: usize) -> Vec<String> {
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for item in evidence {
        for word in item
            .content
            .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
            .map(|word| word.trim_matches('_').to_ascii_lowercase())
        {
            if word.len() < 4 || is_stopword(&word) || word.chars().all(|ch| ch.is_ascii_digit()) {
                continue;
            }
            *counts.entry(word).or_insert(0) += 1;
        }
    }
    let mut ranked = counts.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|(left_word, left_count), (right_word, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| left_word.cmp(right_word))
    });
    ranked
        .into_iter()
        .take(limit)
        .map(|(word, _)| word)
        .collect()
}

fn command_tokens(evidence: &[LoadedEvidence]) -> Vec<String> {
    let mut commands = Vec::new();
    for item in evidence {
        for token in item
            .content
            .split(|ch: char| !ch.is_ascii_alphanumeric())
            .filter(|token| {
                (2..=16).contains(&token.len())
                    && token.chars().any(|ch| ch.is_ascii_alphabetic())
                    && token
                        .chars()
                        .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
            })
        {
            let token = token.to_string();
            if !commands.contains(&token) {
                commands.push(token);
            }
        }
    }
    if commands.is_empty() {
        commands.push("PING".to_string());
    }
    commands
}

fn string_field(value: &serde_json::Value, names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        value
            .get(*name)
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_string)
    })
}

fn string_array_field(value: &serde_json::Value, name: &str) -> Option<Vec<String>> {
    let values = value.get(name)?.as_array()?;
    let strings = values
        .iter()
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    (!strings.is_empty()).then_some(strings)
}

fn slug(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        "item".to_string()
    } else {
        out
    }
}

fn is_stopword(word: &str) -> bool {
    matches!(
        word,
        "about"
            | "after"
            | "before"
            | "build"
            | "client"
            | "clients"
            | "command"
            | "commands"
            | "component"
            | "components"
            | "define"
            | "design"
            | "evidence"
            | "from"
            | "implementation"
            | "important"
            | "should"
            | "stage"
            | "stages"
            | "system"
            | "target"
            | "tests"
            | "this"
            | "with"
            | "would"
    )
}
