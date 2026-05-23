-- Durable advanced reasoning state for ZYAL port workflows.
--
-- Rollback:
--   DROP TABLE `daemon_model_reliability`;
--   DROP TABLE `daemon_memory_capsule`;
--   DROP TABLE `daemon_reasoning_lane`;
--   DROP TABLE `daemon_reasoning_edge`;
--   DROP TABLE `daemon_reasoning_artifact`;

CREATE TABLE `daemon_reasoning_artifact` (
    `id` text PRIMARY KEY NOT NULL,
    `run_id` text NOT NULL,
    `role` text NOT NULL,
    `kind` text NOT NULL,
    `title` text NOT NULL,
    `summary` text NOT NULL,
    `evidence_level` text NOT NULL,
    `confidence` real NOT NULL DEFAULT 0,
    `payload_json` text,
    `content_hash` text NOT NULL,
    `status` text NOT NULL DEFAULT 'candidate',
    `time_created` integer NOT NULL,
    `time_updated` integer NOT NULL,
    CONSTRAINT `fk_daemon_reasoning_artifact_run` FOREIGN KEY (`run_id`) REFERENCES `daemon_run`(`id`) ON DELETE RESTRICT
);--> statement-breakpoint
CREATE INDEX `daemon_reasoning_artifact_run_kind_idx` ON `daemon_reasoning_artifact` (`run_id`, `kind`, `status`);--> statement-breakpoint

CREATE TABLE `daemon_reasoning_edge` (
    `run_id` text NOT NULL,
    `src_artifact_id` text NOT NULL,
    `dst_artifact_id` text NOT NULL,
    `kind` text NOT NULL,
    `weight` real,
    `payload_json` text,
    `time_created` integer NOT NULL,
    PRIMARY KEY (`run_id`, `src_artifact_id`, `dst_artifact_id`, `kind`),
    CONSTRAINT `fk_daemon_reasoning_edge_run` FOREIGN KEY (`run_id`) REFERENCES `daemon_run`(`id`) ON DELETE RESTRICT,
    CONSTRAINT `fk_daemon_reasoning_edge_src` FOREIGN KEY (`src_artifact_id`) REFERENCES `daemon_reasoning_artifact`(`id`) ON DELETE RESTRICT,
    CONSTRAINT `fk_daemon_reasoning_edge_dst` FOREIGN KEY (`dst_artifact_id`) REFERENCES `daemon_reasoning_artifact`(`id`) ON DELETE RESTRICT
);--> statement-breakpoint
CREATE INDEX `daemon_reasoning_edge_dst_idx` ON `daemon_reasoning_edge` (`run_id`, `dst_artifact_id`, `kind`);--> statement-breakpoint

CREATE TABLE `daemon_reasoning_lane` (
    `id` text PRIMARY KEY NOT NULL,
    `run_id` text NOT NULL,
    `role` text NOT NULL,
    `strategy` text NOT NULL,
    `status` text NOT NULL,
    `artifact_ids_json` text NOT NULL DEFAULT '[]',
    `write_scope_json` text NOT NULL DEFAULT '[]',
    `worker_id` text,
    `confidence` real NOT NULL DEFAULT 0,
    `time_created` integer NOT NULL,
    `time_updated` integer NOT NULL,
    CONSTRAINT `fk_daemon_reasoning_lane_run` FOREIGN KEY (`run_id`) REFERENCES `daemon_run`(`id`) ON DELETE RESTRICT
);--> statement-breakpoint
CREATE INDEX `daemon_reasoning_lane_run_status_idx` ON `daemon_reasoning_lane` (`run_id`, `status`);--> statement-breakpoint

CREATE TABLE `daemon_memory_capsule` (
    `id` text PRIMARY KEY NOT NULL,
    `run_id` text NOT NULL,
    `artifact_id` text NOT NULL,
    `scope` text NOT NULL,
    `status` text NOT NULL,
    `summary` text NOT NULL,
    `evidence_level` text NOT NULL,
    `confidence` real NOT NULL DEFAULT 0,
    `payload_json` text,
    `content_hash` text NOT NULL,
    `time_created` integer NOT NULL,
    `time_updated` integer NOT NULL,
    CONSTRAINT `fk_daemon_memory_capsule_run` FOREIGN KEY (`run_id`) REFERENCES `daemon_run`(`id`) ON DELETE RESTRICT,
    CONSTRAINT `fk_daemon_memory_capsule_artifact` FOREIGN KEY (`artifact_id`) REFERENCES `daemon_reasoning_artifact`(`id`) ON DELETE RESTRICT
);--> statement-breakpoint
CREATE INDEX `daemon_memory_capsule_run_status_idx` ON `daemon_memory_capsule` (`run_id`, `status`, `scope`);--> statement-breakpoint

CREATE TABLE `daemon_model_reliability` (
    `model_id` text NOT NULL,
    `role` text NOT NULL,
    `task_kind` text NOT NULL,
    `success_count` integer NOT NULL DEFAULT 0,
    `failure_count` integer NOT NULL DEFAULT 0,
    `winner_count` integer NOT NULL DEFAULT 0,
    `total_latency_ms` integer NOT NULL DEFAULT 0,
    `total_cost_usd` real NOT NULL DEFAULT 0,
    `score` real NOT NULL DEFAULT 0,
    `time_created` integer NOT NULL,
    `time_updated` integer NOT NULL,
    PRIMARY KEY (`model_id`, `role`, `task_kind`)
);--> statement-breakpoint
CREATE INDEX `daemon_model_reliability_score_idx` ON `daemon_model_reliability` (`task_kind`, `score`);
