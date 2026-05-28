//! Phase-DAG traversal helpers for [`SuperReasoningPlan`].
//!
//! All graph queries (topological order, parallel waves, ready-phase lookup)
//! share the same `(indegree, children)` precompute that lives at the bottom
//! of this file. Cycles are reported as `RuntimeError::invalid`.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::SuperReasoningPlan;
use crate::error::{RuntimeError, RuntimeResult};

impl SuperReasoningPlan {
    /// Return phase ids in deterministic topological order.
    pub fn topological_phase_ids(&self) -> RuntimeResult<Vec<String>> {
        let (mut indegree, mut children) = self.dependency_maps()?;
        let mut queue: VecDeque<String> = indegree
            .iter()
            .filter(|&(_id, degree)| *degree == 0)
            .map(|(id, _degree)| id.clone())
            .collect();
        let mut out = Vec::with_capacity(self.phases.len());
        while let Some(id) = queue.pop_front() {
            out.push(id.clone());
            if let Some(next) = children.remove(&id) {
                for child in next {
                    let degree = match indegree.get_mut(&child) {
                        Some(d) => d,
                        None => return Err(RuntimeError::invalid("dependency map corrupt")),
                    };
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(child);
                    }
                }
            }
        }
        if out.len() != self.phases.len() {
            return Err(RuntimeError::invalid(
                "super reasoning phase dependency graph contains a cycle",
            ));
        }
        Ok(out)
    }

    /// Return deterministic parallel waves from the phase DAG.
    pub fn parallel_waves(&self) -> RuntimeResult<Vec<Vec<String>>> {
        let mut remaining: BTreeMap<String, BTreeSet<String>> = self
            .phases
            .iter()
            .map(|phase| {
                (
                    phase.id.clone(),
                    phase.depends_on.iter().cloned().collect::<BTreeSet<_>>(),
                )
            })
            .collect();
        let valid_ids: BTreeSet<String> = remaining.keys().cloned().collect();
        for (id, deps) in &remaining {
            for dep in deps {
                if !valid_ids.contains(dep) {
                    return Err(RuntimeError::invalid(format!(
                        "phase '{id}' depends on unknown phase '{dep}'"
                    )));
                }
            }
        }

        let mut waves = Vec::new();
        while !remaining.is_empty() {
            let wave: Vec<String> = remaining
                .iter()
                .filter(|&(_id, deps)| deps.is_empty())
                .map(|(id, _deps)| id.clone())
                .collect();
            if wave.is_empty() {
                return Err(RuntimeError::invalid(
                    "super reasoning phase dependency graph contains a cycle",
                ));
            }
            for id in &wave {
                remaining.remove(id);
            }
            for deps in remaining.values_mut() {
                for id in &wave {
                    deps.remove(id);
                }
            }
            waves.push(wave);
        }
        Ok(waves)
    }

    /// Return phases that are runnable given a set of completed phase ids.
    pub fn ready_phase_ids<I, S>(&self, completed_phase_ids: I) -> RuntimeResult<Vec<String>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let completed: BTreeSet<String> = completed_phase_ids
            .into_iter()
            .map(|s| s.as_ref().to_string())
            .collect();
        let valid_ids: BTreeSet<String> = self.phases.iter().map(|p| p.id.clone()).collect();
        for id in &completed {
            if !valid_ids.contains(id) {
                return Err(RuntimeError::invalid(format!(
                    "completed phase '{id}' is not in this plan"
                )));
            }
        }
        Ok(self
            .phases
            .iter()
            .filter(|phase| {
                !completed.contains(&phase.id)
                    && phase.depends_on.iter().all(|dep| completed.contains(dep))
            })
            .map(|phase| phase.id.clone())
            .collect())
    }

    // `(indegree-by-phase-id, children-by-phase-id)`. clippy::type_complexity
    // suggests a type alias; we keep the tuple inline because it's used only
    // by the immediate caller (`validate`) and aliasing would obscure intent
    // for the (correct) reading "topo data, separated by direction."
    #[allow(clippy::type_complexity)]
    pub(super) fn dependency_maps(
        &self,
    ) -> RuntimeResult<(BTreeMap<String, usize>, BTreeMap<String, Vec<String>>)> {
        let mut ids = BTreeSet::new();
        for phase in &self.phases {
            if !ids.insert(phase.id.clone()) {
                return Err(RuntimeError::invalid(format!(
                    "duplicate phase id '{}'",
                    phase.id
                )));
            }
        }
        let mut indegree: BTreeMap<String, usize> = ids.iter().map(|id| (id.clone(), 0)).collect();
        let mut children: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for phase in &self.phases {
            // Dedupe deps per phase — a duplicate entry (e.g. `depends_on:
            // ["p01", "p01"]`) would inflate indegree past what the topo
            // walk can decrement, falsely reporting a cycle.
            let unique_deps: std::collections::BTreeSet<&String> =
                phase.depends_on.iter().collect();
            for dep in unique_deps {
                if !ids.contains(dep) {
                    return Err(RuntimeError::invalid(format!(
                        "phase '{}' depends on unknown phase '{}'",
                        phase.id, dep
                    )));
                }
                let degree = match indegree.get_mut(&phase.id) {
                    Some(d) => d,
                    None => return Err(RuntimeError::invalid("dependency map corrupt")),
                };
                *degree += 1;
                children
                    .entry(dep.clone())
                    .or_default()
                    .push(phase.id.clone());
            }
        }
        for child_list in children.values_mut() {
            child_list.sort();
        }
        Ok((indegree, children))
    }
}
