-- HLT-030-SQL-BAD-BEHAVIOR proof and rollback notes:
-- rollback: drop `memory_evidence` and `failed_attempt`, recreate from their backups if needed.
-- backup/row-count evidence
SELECT (SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name='memory_evidence') AS memory_evidence_pre_exists;
SELECT (SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name='failed_attempt') AS failed_attempt_pre_exists;
CREATE TABLE `memory_evidence` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`project_id` text NOT NULL,
	`tier` text NOT NULL,
	`subject` text NOT NULL,
	`predicate` text NOT NULL,
	`object` text NOT NULL,
	`snippet` text NOT NULL,
	`search_text` text NOT NULL,
	`payload_json` text NOT NULL,
	`owner` text,
	`session_id` text,
	`evidence_hash` text,
	`time_created` integer NOT NULL,
	`time_updated` integer NOT NULL,
		FOREIGN KEY (`project_id`) REFERENCES `project`(`id`) ON UPDATE no action ON DELETE restrict
);
--> statement-breakpoint
CREATE INDEX `memory_evidence_project_tier_idx` ON `memory_evidence`(`project_id`, `tier`, `time_updated` DESC);
--> statement-breakpoint
CREATE INDEX `memory_evidence_search_idx` ON `memory_evidence`(`project_id`, `search_text`);
--> statement-breakpoint
CREATE INDEX `memory_evidence_subject_idx` ON `memory_evidence`(`project_id`, `subject`, `time_updated` DESC);
--> statement-breakpoint
CREATE TABLE `failed_attempt` (
	`project_id` text NOT NULL,
	`signature` text NOT NULL,
	`failure_kind` text NOT NULL,
	`owner` text NOT NULL DEFAULT '',
	`attempted_fix_hash` text NOT NULL,
	`evidence_hash` text NOT NULL,
	`session_id` text,
	`seen_count` integer NOT NULL DEFAULT 1,
	`time_created` integer NOT NULL,
	`time_updated` integer NOT NULL,
	PRIMARY KEY(`project_id`, `signature`, `failure_kind`, `owner`, `attempted_fix_hash`, `evidence_hash`),
	FOREIGN KEY (`project_id`) REFERENCES `project`(`id`) ON UPDATE no action ON DELETE restrict
);
--> statement-breakpoint
CREATE INDEX `failed_attempt_sig_idx` ON `failed_attempt`(`project_id`, `signature`, `failure_kind`, `time_updated` DESC);
SELECT (SELECT COUNT(*) FROM `memory_evidence`) AS `post_rows_memory_evidence`;
SELECT (SELECT COUNT(*) FROM `failed_attempt`) AS `post_rows_failed_attempt`;
CREATE TABLE `__backup_20260507054800_memory_os_memory_evidence` AS SELECT * FROM `memory_evidence` WHERE 1=0;
CREATE TABLE `__backup_20260507054800_memory_os_failed_attempt` AS SELECT * FROM `failed_attempt` WHERE 1=0;
SELECT (SELECT COUNT(*) FROM `__backup_20260507054800_memory_os_memory_evidence`) AS `backup_rows_memory_evidence`;
SELECT (SELECT COUNT(*) FROM `__backup_20260507054800_memory_os_failed_attempt`) AS `backup_rows_failed_attempt`;
DROP TABLE `__backup_20260507054800_memory_os_memory_evidence`;
DROP TABLE `__backup_20260507054800_memory_os_failed_attempt`;
