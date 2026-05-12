-- Rollback (drop in reverse FK order):
--   DROP TABLE `daemon_artifact`; DROP TABLE `daemon_worker`;
--   DROP TABLE `daemon_task_memory`; DROP TABLE `daemon_task_pass`;
--   DROP TABLE `daemon_task`; DROP TABLE `daemon_event`;
--   DROP TABLE `daemon_iteration`; DROP TABLE `daemon_run`;
-- Pre-flight: SELECT (SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name LIKE 'daemon_%') AS pre_count;
CREATE TABLE `daemon_run` (
	`id` text PRIMARY KEY NOT NULL,
	`root_session_id` text NOT NULL REFERENCES `session`(`id`) ON DELETE restrict,
	`active_session_id` text NOT NULL REFERENCES `session`(`id`) ON DELETE restrict,
	`status` text NOT NULL,
	`phase` text NOT NULL,
	`spec_json` text NOT NULL,
	`spec_hash` text NOT NULL,
	`iteration` integer NOT NULL,
	`epoch` integer NOT NULL,
	`last_error` text,
	`last_exit_result_json` text,
	`stopped_at` integer,
	`time_created` integer NOT NULL,
	`time_updated` integer NOT NULL
);
--> statement-breakpoint
CREATE INDEX `daemon_run_root_idx` ON `daemon_run` (`root_session_id`);
--> statement-breakpoint
CREATE INDEX `daemon_run_active_idx` ON `daemon_run` (`active_session_id`);
--> statement-breakpoint
CREATE INDEX `daemon_run_status_idx` ON `daemon_run` (`status`);
--> statement-breakpoint
CREATE TABLE `daemon_iteration` (
	`run_id` text NOT NULL REFERENCES `daemon_run`(`id`) ON DELETE restrict,
	`iteration` integer NOT NULL,
	`session_id` text NOT NULL REFERENCES `session`(`id`) ON DELETE restrict,
	`terminal_reason` text NOT NULL,
	`result_json` text NOT NULL,
	`token_usage_json` text,
	`cost` real,
	`checkpoint_sha` text,
	`time_created` integer NOT NULL,
	`time_updated` integer NOT NULL,
	PRIMARY KEY(`run_id`, `iteration`)
);
--> statement-breakpoint
CREATE INDEX `daemon_iteration_run_idx` ON `daemon_iteration` (`run_id`);
--> statement-breakpoint
CREATE TABLE `daemon_event` (
	`id` text PRIMARY KEY NOT NULL,
	`run_id` text NOT NULL REFERENCES `daemon_run`(`id`) ON DELETE restrict,
	`iteration` integer NOT NULL,
	`event_type` text NOT NULL,
	`payload_json` text NOT NULL,
	`time_created` integer NOT NULL,
	`time_updated` integer NOT NULL
);
--> statement-breakpoint
CREATE INDEX `daemon_event_run_idx` ON `daemon_event` (`run_id`, `time_created`);
--> statement-breakpoint
CREATE TABLE `daemon_task` (
	`id` text PRIMARY KEY NOT NULL,
	`run_id` text NOT NULL REFERENCES `daemon_run`(`id`) ON DELETE restrict,
	`external_id` text,
	`title` text NOT NULL,
	`body_json` text NOT NULL,
	`status` text NOT NULL,
	`lane` text DEFAULT 'normal' NOT NULL,
	`phase` text DEFAULT 'queued' NOT NULL,
	`difficulty_score` real DEFAULT 0 NOT NULL,
	`risk_score` real DEFAULT 0 NOT NULL,
	`readiness_score` real DEFAULT 0 NOT NULL,
	`implementation_confidence` real DEFAULT 0 NOT NULL,
	`verification_confidence` real DEFAULT 0 NOT NULL,
	`attempt_count` integer DEFAULT 0 NOT NULL,
	`no_progress_count` integer DEFAULT 0 NOT NULL,
	`incubator_round` integer DEFAULT 0 NOT NULL,
	`incubator_status` text DEFAULT 'none' NOT NULL,
	`accepted_artifact_id` text,
	`last_assessment_json` text,
	`promotion_result_json` text,
	`blocked_reason` text,
	`priority` integer NOT NULL,
	`lease_worker_id` text,
	`lease_expires_at` integer,
	`locked_paths_json` text,
	`evidence_json` text,
	`time_created` integer NOT NULL,
	`time_updated` integer NOT NULL
);
--> statement-breakpoint
CREATE INDEX `daemon_task_run_status_idx` ON `daemon_task` (`run_id`, `status`, `priority`);
--> statement-breakpoint
CREATE INDEX `daemon_task_lane_status_idx` ON `daemon_task` (`run_id`, `lane`, `status`, `priority`);
--> statement-breakpoint
CREATE INDEX `daemon_task_lease_idx` ON `daemon_task` (`lease_expires_at`);
--> statement-breakpoint
CREATE TABLE `daemon_task_pass` (
	`id` text PRIMARY KEY NOT NULL,
	`run_id` text NOT NULL REFERENCES `daemon_run`(`id`) ON DELETE restrict,
	`task_id` text NOT NULL REFERENCES `daemon_task`(`id`) ON DELETE restrict,
	`pass_number` integer NOT NULL,
	`pass_type` text NOT NULL,
	`context_mode` text NOT NULL,
	`agent` text,
	`session_id` text REFERENCES `session`(`id`) ON DELETE set null,
	`worker_id` text,
	`status` text NOT NULL,
	`started_at` integer,
	`ended_at` integer,
	`worktree_path` text,
	`worktree_branch` text,
	`cleanup_status` text NOT NULL DEFAULT 'pending',
	`input_artifact_ids_json` text,
	`output_artifact_ids_json` text,
	`result_json` text,
	`score_json` text,
	`error_json` text,
	`time_created` integer NOT NULL,
	`time_updated` integer NOT NULL
);
--> statement-breakpoint
CREATE INDEX `daemon_task_pass_task_idx` ON `daemon_task_pass` (`run_id`, `task_id`, `pass_number`);
--> statement-breakpoint
CREATE INDEX `daemon_task_pass_status_idx` ON `daemon_task_pass` (`run_id`, `status`);
--> statement-breakpoint
CREATE TABLE `daemon_task_memory` (
	`id` text PRIMARY KEY NOT NULL,
	`run_id` text NOT NULL REFERENCES `daemon_run`(`id`) ON DELETE restrict,
	`task_id` text NOT NULL REFERENCES `daemon_task`(`id`) ON DELETE restrict,
	`kind` text NOT NULL,
	`title` text NOT NULL,
	`summary` text NOT NULL,
	`payload_json` text,
	`source_pass_id` text REFERENCES `daemon_task_pass`(`id`) ON DELETE set null,
	`importance` real DEFAULT 0.5 NOT NULL,
	`confidence` real DEFAULT 0.5 NOT NULL,
	`time_created` integer NOT NULL,
	`time_updated` integer NOT NULL
);
--> statement-breakpoint
CREATE INDEX `daemon_task_memory_task_idx` ON `daemon_task_memory` (`run_id`, `task_id`, `time_created`);
--> statement-breakpoint
CREATE INDEX `daemon_task_memory_kind_idx` ON `daemon_task_memory` (`run_id`, `task_id`, `kind`);
--> statement-breakpoint
CREATE TABLE `daemon_worker` (
	`id` text PRIMARY KEY NOT NULL,
	`run_id` text NOT NULL REFERENCES `daemon_run`(`id`) ON DELETE restrict,
	`role` text NOT NULL,
	`session_id` text REFERENCES `session`(`id`) ON DELETE set null,
	`worktree_path` text,
	`branch` text,
	`status` text NOT NULL,
	`lease_task_id` text,
	`last_heartbeat_at` integer,
	`time_created` integer NOT NULL,
	`time_updated` integer NOT NULL
);
--> statement-breakpoint
CREATE INDEX `daemon_worker_run_idx` ON `daemon_worker` (`run_id`, `status`);
--> statement-breakpoint
CREATE TABLE `daemon_artifact` (
	`id` text PRIMARY KEY NOT NULL,
	`run_id` text NOT NULL REFERENCES `daemon_run`(`id`) ON DELETE restrict,
	`task_id` text REFERENCES `daemon_task`(`id`) ON DELETE restrict,
	`pass_id` text REFERENCES `daemon_task_pass`(`id`) ON DELETE set null,
	`kind` text NOT NULL,
	`path_or_ref` text NOT NULL,
	`sha` text,
	`payload_json` text,
	`time_created` integer NOT NULL,
	`time_updated` integer NOT NULL
);
--> statement-breakpoint
CREATE INDEX `daemon_artifact_run_idx` ON `daemon_artifact` (`run_id`);
--> statement-breakpoint
CREATE INDEX `daemon_artifact_task_idx` ON `daemon_artifact` (`run_id`, `task_id`);
--> statement-breakpoint
CREATE INDEX `daemon_artifact_pass_idx` ON `daemon_artifact` (`run_id`, `pass_id`);
