-- Durable state for generic ZYAL port workflows.
--
-- Rollback:
--   DROP TABLE `daemon_model_outcome`;
--   DROP TABLE `daemon_repo_graph_edge`;
--   DROP TABLE `daemon_repo_graph_node`;
--   DROP TABLE `daemon_perf_budget`;
--   DROP TABLE `daemon_parity_result`;
--   DROP TABLE `daemon_parity_run`;
--   DROP TABLE `daemon_parity_case`;
--   DROP TABLE `daemon_port_task`;
--   DROP TABLE `daemon_port_phase`;
--   DROP TABLE `daemon_port_target`;

CREATE TABLE `daemon_port_target` (
    `id` text PRIMARY KEY NOT NULL,
    `run_id` text NOT NULL,
    `target` text NOT NULL,
    `replacement` text NOT NULL,
    `target_repo` text,
    `replacement_repo` text,
    `request` text NOT NULL,
    `status` text NOT NULL DEFAULT 'drafting',
    `current_phase_id` text,
    `worker_cap` integer NOT NULL DEFAULT 1,
    `last_audit_score` real,
    `last_parity_report_json` text,
    `last_perf_gap_json` text,
    `rollback_status` text NOT NULL DEFAULT 'clean',
    `quarantine_status` text NOT NULL DEFAULT 'none',
    `time_created` integer NOT NULL,
    `time_updated` integer NOT NULL,
    CONSTRAINT `fk_daemon_port_target_run` FOREIGN KEY (`run_id`) REFERENCES `daemon_run`(`id`) ON DELETE RESTRICT
);--> statement-breakpoint
CREATE INDEX `daemon_port_target_run_idx` ON `daemon_port_target` (`run_id`, `status`);--> statement-breakpoint

CREATE TABLE `daemon_port_phase` (
    `id` text PRIMARY KEY NOT NULL,
    `run_id` text NOT NULL,
    `target_id` text NOT NULL,
    `ordinal` integer NOT NULL,
    `name` text NOT NULL,
    `status` text NOT NULL DEFAULT 'drafting',
    `strategy` text NOT NULL DEFAULT 'generic',
    `plan_json` text,
    `task_count` integer NOT NULL DEFAULT 0,
    `last_audit_score` real,
    `last_parity_report_json` text,
    `time_created` integer NOT NULL,
    `time_updated` integer NOT NULL,
    CONSTRAINT `fk_daemon_port_phase_run` FOREIGN KEY (`run_id`) REFERENCES `daemon_run`(`id`) ON DELETE RESTRICT,
    CONSTRAINT `fk_daemon_port_phase_target` FOREIGN KEY (`target_id`) REFERENCES `daemon_port_target`(`id`) ON DELETE RESTRICT
);--> statement-breakpoint
CREATE UNIQUE INDEX `daemon_port_phase_target_ord_idx` ON `daemon_port_phase` (`target_id`, `ordinal`);--> statement-breakpoint
CREATE INDEX `daemon_port_phase_run_status_idx` ON `daemon_port_phase` (`run_id`, `status`);--> statement-breakpoint

CREATE TABLE `daemon_port_task` (
    `id` text PRIMARY KEY NOT NULL,
    `run_id` text NOT NULL,
    `phase_id` text NOT NULL,
    `title` text NOT NULL,
    `status` text NOT NULL DEFAULT 'queued',
    `worker_id` text,
    `branch` text,
    `write_scope_json` text NOT NULL DEFAULT '[]',
    `proof_lane` text,
    `attempt_count` integer NOT NULL DEFAULT 0,
    `rollback_status` text NOT NULL DEFAULT 'clean',
    `quarantine_reason` text,
    `last_error` text,
    `time_created` integer NOT NULL,
    `time_updated` integer NOT NULL,
    CONSTRAINT `fk_daemon_port_task_run` FOREIGN KEY (`run_id`) REFERENCES `daemon_run`(`id`) ON DELETE RESTRICT,
    CONSTRAINT `fk_daemon_port_task_phase` FOREIGN KEY (`phase_id`) REFERENCES `daemon_port_phase`(`id`) ON DELETE RESTRICT
);--> statement-breakpoint
CREATE INDEX `daemon_port_task_phase_status_idx` ON `daemon_port_task` (`phase_id`, `status`);--> statement-breakpoint
CREATE INDEX `daemon_port_task_run_worker_idx` ON `daemon_port_task` (`run_id`, `worker_id`);--> statement-breakpoint

CREATE TABLE `daemon_parity_case` (
    `id` text PRIMARY KEY NOT NULL,
    `run_id` text NOT NULL,
    `target_id` text NOT NULL,
    `tags_json` text NOT NULL DEFAULT '[]',
    `target_kind` text NOT NULL,
    `steps_json` text NOT NULL DEFAULT '[]',
    `perf_json` text,
    `approved` integer NOT NULL DEFAULT 0,
    `time_created` integer NOT NULL,
    `time_updated` integer NOT NULL,
    CONSTRAINT `fk_daemon_parity_case_run` FOREIGN KEY (`run_id`) REFERENCES `daemon_run`(`id`) ON DELETE RESTRICT,
    CONSTRAINT `fk_daemon_parity_case_target` FOREIGN KEY (`target_id`) REFERENCES `daemon_port_target`(`id`) ON DELETE RESTRICT
);--> statement-breakpoint
CREATE INDEX `daemon_parity_case_target_idx` ON `daemon_parity_case` (`target_id`, `approved`);--> statement-breakpoint

CREATE TABLE `daemon_parity_run` (
    `id` text PRIMARY KEY NOT NULL,
    `run_id` text NOT NULL,
    `target_id` text NOT NULL,
    `case_count` integer NOT NULL DEFAULT 0,
    `status` text NOT NULL DEFAULT 'running',
    `report_path` text,
    `started_at` integer,
    `ended_at` integer,
    `summary_json` text,
    `time_created` integer NOT NULL,
    `time_updated` integer NOT NULL,
    CONSTRAINT `fk_daemon_parity_run_run` FOREIGN KEY (`run_id`) REFERENCES `daemon_run`(`id`) ON DELETE RESTRICT,
    CONSTRAINT `fk_daemon_parity_run_target` FOREIGN KEY (`target_id`) REFERENCES `daemon_port_target`(`id`) ON DELETE RESTRICT
);--> statement-breakpoint
CREATE INDEX `daemon_parity_run_target_idx` ON `daemon_parity_run` (`target_id`, `status`);--> statement-breakpoint

CREATE TABLE `daemon_parity_result` (
    `id` text PRIMARY KEY NOT NULL,
    `parity_run_id` text NOT NULL,
    `case_id` text NOT NULL,
    `target_name` text NOT NULL,
    `status` text NOT NULL,
    `skipped` integer NOT NULL DEFAULT 0,
    `duration_ms` integer,
    `perf_json` text,
    `message` text,
    `time_created` integer NOT NULL,
    CONSTRAINT `fk_daemon_parity_result_run` FOREIGN KEY (`parity_run_id`) REFERENCES `daemon_parity_run`(`id`) ON DELETE RESTRICT,
    CONSTRAINT `fk_daemon_parity_result_case` FOREIGN KEY (`case_id`) REFERENCES `daemon_parity_case`(`id`) ON DELETE RESTRICT
);--> statement-breakpoint
CREATE INDEX `daemon_parity_result_run_status_idx` ON `daemon_parity_result` (`parity_run_id`, `status`);--> statement-breakpoint

CREATE TABLE `daemon_perf_budget` (
    `id` text PRIMARY KEY NOT NULL,
    `run_id` text NOT NULL,
    `case_id` text NOT NULL,
    `metric` text NOT NULL,
    `max_ratio` real,
    `baseline_value` real,
    `candidate_value` real,
    `status` text NOT NULL DEFAULT 'pending',
    `time_created` integer NOT NULL,
    `time_updated` integer NOT NULL,
    CONSTRAINT `fk_daemon_perf_budget_run` FOREIGN KEY (`run_id`) REFERENCES `daemon_run`(`id`) ON DELETE RESTRICT,
    CONSTRAINT `fk_daemon_perf_budget_case` FOREIGN KEY (`case_id`) REFERENCES `daemon_parity_case`(`id`) ON DELETE RESTRICT
);--> statement-breakpoint
CREATE INDEX `daemon_perf_budget_case_idx` ON `daemon_perf_budget` (`case_id`, `metric`);--> statement-breakpoint

CREATE TABLE `daemon_repo_graph_node` (
    `id` text PRIMARY KEY NOT NULL,
    `run_id` text NOT NULL,
    `kind` text NOT NULL,
    `key` text NOT NULL,
    `label` text NOT NULL,
    `payload_json` text,
    `time_created` integer NOT NULL,
    `time_updated` integer NOT NULL,
    CONSTRAINT `fk_daemon_repo_graph_node_run` FOREIGN KEY (`run_id`) REFERENCES `daemon_run`(`id`) ON DELETE RESTRICT
);--> statement-breakpoint
CREATE UNIQUE INDEX `daemon_repo_graph_node_key_idx` ON `daemon_repo_graph_node` (`run_id`, `kind`, `key`);--> statement-breakpoint

CREATE TABLE `daemon_repo_graph_edge` (
    `run_id` text NOT NULL,
    `src_node_id` text NOT NULL,
    `dst_node_id` text NOT NULL,
    `kind` text NOT NULL,
    `payload_json` text,
    `time_created` integer NOT NULL,
    PRIMARY KEY (`run_id`, `src_node_id`, `dst_node_id`, `kind`),
    CONSTRAINT `fk_daemon_repo_graph_edge_run` FOREIGN KEY (`run_id`) REFERENCES `daemon_run`(`id`) ON DELETE RESTRICT,
    CONSTRAINT `fk_daemon_repo_graph_edge_src` FOREIGN KEY (`src_node_id`) REFERENCES `daemon_repo_graph_node`(`id`) ON DELETE RESTRICT,
    CONSTRAINT `fk_daemon_repo_graph_edge_dst` FOREIGN KEY (`dst_node_id`) REFERENCES `daemon_repo_graph_node`(`id`) ON DELETE RESTRICT
);--> statement-breakpoint
CREATE INDEX `daemon_repo_graph_edge_dst_idx` ON `daemon_repo_graph_edge` (`run_id`, `dst_node_id`, `kind`);--> statement-breakpoint

CREATE TABLE `daemon_model_outcome` (
    `id` text PRIMARY KEY NOT NULL,
    `run_id` text NOT NULL,
    `task_id` text,
    `model_id` text NOT NULL,
    `role` text NOT NULL,
    `cost_usd` real,
    `latency_ms` integer,
    `status` text NOT NULL,
    `reviewer_score` real,
    `winner` integer NOT NULL DEFAULT 0,
    `payload_json` text,
    `time_created` integer NOT NULL,
    `time_updated` integer NOT NULL,
    CONSTRAINT `fk_daemon_model_outcome_run` FOREIGN KEY (`run_id`) REFERENCES `daemon_run`(`id`) ON DELETE RESTRICT,
    CONSTRAINT `fk_daemon_model_outcome_task` FOREIGN KEY (`task_id`) REFERENCES `daemon_port_task`(`id`) ON DELETE SET NULL
);--> statement-breakpoint
CREATE INDEX `daemon_model_outcome_run_task_idx` ON `daemon_model_outcome` (`run_id`, `task_id`, `winner`);
