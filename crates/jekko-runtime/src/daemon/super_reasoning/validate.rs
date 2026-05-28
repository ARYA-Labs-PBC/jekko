//! Plan validation logic for [`SuperReasoningPlan`].
//!
//! Runs before daemon registration / execution and rejects plans with missing
//! required fields, illegal counts, or graph cycles. Cycle detection is
//! delegated to [`super::SuperReasoningPlan::topological_phase_ids`].

use std::collections::BTreeSet;

use super::{SuperReasoningPlan, DEFAULT_MAX_WORKERS, SUPER_REASONING_SCHEMA_VERSION};
use crate::error::{RuntimeError, RuntimeResult};

impl SuperReasoningPlan {
    /// Validate the plan before daemon registration or execution.
    pub fn validate(&self) -> RuntimeResult<()> {
        if self.schema_version != SUPER_REASONING_SCHEMA_VERSION {
            return Err(RuntimeError::invalid(format!(
                "unsupported super reasoning schema '{}', expected '{}'",
                self.schema_version, SUPER_REASONING_SCHEMA_VERSION
            )));
        }
        if self.mission_id.trim().is_empty() {
            return Err(RuntimeError::invalid(
                "super reasoning mission_id is required",
            ));
        }
        if self.objective.trim().is_empty() {
            return Err(RuntimeError::invalid(
                "super reasoning objective is required",
            ));
        }
        if self.phase_count.min == 0 || self.phase_count.min > self.phase_count.max {
            return Err(RuntimeError::invalid(
                "phase_count must have min > 0 and min <= max",
            ));
        }
        let phase_len = self.phases.len();
        if phase_len < self.phase_count.min || phase_len > self.phase_count.max {
            return Err(RuntimeError::invalid(format!(
                "super reasoning plans require {}-{} phases; got {}",
                self.phase_count.min, self.phase_count.max, phase_len
            )));
        }
        if self.swarm.max_workers == 0 || self.swarm.max_workers > DEFAULT_MAX_WORKERS {
            return Err(RuntimeError::invalid(format!(
                "swarm.max_workers must be between 1 and {DEFAULT_MAX_WORKERS}"
            )));
        }
        if self.swarm.weak_agent_redundancy == 0 {
            return Err(RuntimeError::invalid(
                "swarm.weak_agent_redundancy must be at least 1",
            ));
        }
        if self.swarm.critic_ratio_percent > 100 {
            return Err(RuntimeError::invalid(
                "swarm.critic_ratio_percent cannot exceed 100",
            ));
        }

        let mut ids = BTreeSet::new();
        for phase in &self.phases {
            if phase.id.trim().is_empty() {
                return Err(RuntimeError::invalid("phase id is required"));
            }
            if !ids.insert(phase.id.clone()) {
                return Err(RuntimeError::invalid(format!(
                    "duplicate phase id '{}'",
                    phase.id
                )));
            }
            if phase.name.trim().is_empty() || phase.objective.trim().is_empty() {
                return Err(RuntimeError::invalid(format!(
                    "phase '{}' requires name and objective",
                    phase.id
                )));
            }
            if phase.workers == 0 || phase.workers > self.swarm.max_workers {
                return Err(RuntimeError::invalid(format!(
                    "phase '{}' workers must be between 1 and swarm.max_workers ({})",
                    phase.id, self.swarm.max_workers
                )));
            }
            if phase.tasks.is_empty() {
                return Err(RuntimeError::invalid(format!(
                    "phase '{}' must declare at least one task",
                    phase.id
                )));
            }
            if phase.lanes.is_empty() {
                return Err(RuntimeError::invalid(format!(
                    "phase '{}' must declare at least one reasoning lane",
                    phase.id
                )));
            }
            if phase.acceptance.is_empty() {
                return Err(RuntimeError::invalid(format!(
                    "phase '{}' must declare at least one acceptance gate",
                    phase.id
                )));
            }
            for dep in &phase.depends_on {
                if dep == &phase.id {
                    return Err(RuntimeError::invalid(format!(
                        "phase '{}' cannot depend on itself",
                        phase.id
                    )));
                }
            }
        }

        let _ = self.topological_phase_ids()?;

        if let Some(parity) = &self.parity {
            if parity.enabled {
                if parity.workflows.is_empty() {
                    return Err(RuntimeError::invalid(
                        "enabled parity policy requires at least one workflow",
                    ));
                }
                if parity.reference_command.trim().is_empty()
                    || parity.candidate_command.trim().is_empty()
                    || parity.manifest.trim().is_empty()
                    || parity.oracle.trim().is_empty()
                {
                    return Err(RuntimeError::invalid(
                        "enabled parity policy requires reference_command, candidate_command, manifest, and oracle",
                    ));
                }
            }
        }

        Ok(())
    }
}
