//! Durable generic-port workflow tables.

mod graph_model;
mod parity;
mod rows;
mod target_task;

pub use graph_model::{
    list_model_outcomes_for_run, list_repo_graph_edges_for_run, list_repo_graph_nodes_for_run,
    upsert_model_outcome, upsert_repo_graph_edge, upsert_repo_graph_node,
};
pub use parity::{
    insert_parity_result, list_parity_cases_for_target, list_parity_results_for_run,
    list_parity_runs_for_target, upsert_parity_case, upsert_parity_run, upsert_perf_budget,
};
pub use rows::{
    ModelOutcomeRow, ParityCaseRow, ParityResultRow, ParityRunRow, PerfBudgetRow, PortPhaseRow,
    PortTargetRow, PortTaskRow, RepoGraphEdgeRow, RepoGraphNodeRow,
};
pub use target_task::{
    get_port_target, get_port_task, list_port_phases_for_target, list_port_targets_for_run,
    list_port_tasks_for_phase, upsert_port_phase, upsert_port_target, upsert_port_task,
};
