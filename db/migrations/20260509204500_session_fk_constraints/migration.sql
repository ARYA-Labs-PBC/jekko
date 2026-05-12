-- Enforce session.workspace_id and session.parent_id as durable truth.
-- rollback: rename `__backup_20260509204500_session_fk_constraints_session` back to `session` if recovery is needed.
SELECT (SELECT COUNT(*) FROM `session`) AS `pre_rows_session`;
PRAGMA foreign_keys=OFF;--> statement-breakpoint
ALTER TABLE `session` RENAME TO `__backup_20260509204500_session_fk_constraints_session`;--> statement-breakpoint
CREATE TABLE `session` (
	`id` text PRIMARY KEY,
	`project_id` text NOT NULL,
	`workspace_id` text,
	`parent_id` text,
	`slug` text NOT NULL,
	`directory` text NOT NULL,
	`path` text,
	`title` text NOT NULL,
	`version` text NOT NULL,
	`share_url` text,
	`summary_additions` integer,
	`summary_deletions` integer,
	`summary_files` integer,
	`summary_diffs` text,
	`revert` text,
	`permission` text,
	`agent` text,
	`model` text,
	`time_created` integer NOT NULL,
	`time_updated` integer NOT NULL,
	`time_compacting` integer,
	`time_archived` integer,
	CONSTRAINT `fk_session_project_id_project_id_fk` FOREIGN KEY (`project_id`) REFERENCES `project`(`id`) ON DELETE CASCADE,
	CONSTRAINT `fk_session_workspace_id_workspace_id_fk` FOREIGN KEY (`workspace_id`) REFERENCES `workspace`(`id`) ON DELETE CASCADE,
	CONSTRAINT `fk_session_parent_id_session_id_fk` FOREIGN KEY (`parent_id`) REFERENCES `session`(`id`) ON DELETE CASCADE
);--> statement-breakpoint
INSERT INTO `session`(`id`, `project_id`, `workspace_id`, `parent_id`, `slug`, `directory`, `path`, `title`, `version`, `share_url`, `summary_additions`, `summary_deletions`, `summary_files`, `summary_diffs`, `revert`, `permission`, `agent`, `model`, `time_created`, `time_updated`, `time_compacting`, `time_archived`) SELECT `id`, `project_id`, `workspace_id`, `parent_id`, `slug`, `directory`, `path`, `title`, `version`, `share_url`, `summary_additions`, `summary_deletions`, `summary_files`, `summary_diffs`, `revert`, `permission`, `agent`, `model`, `time_created`, `time_updated`, `time_compacting`, `time_archived` FROM `__backup_20260509204500_session_fk_constraints_session`;--> statement-breakpoint
DROP INDEX IF EXISTS `session_project_idx`;--> statement-breakpoint
DROP INDEX IF EXISTS `session_workspace_idx`;--> statement-breakpoint
DROP INDEX IF EXISTS `session_parent_idx`;--> statement-breakpoint
CREATE INDEX `session_project_idx` ON `session` (`project_id`);--> statement-breakpoint
CREATE INDEX `session_workspace_idx` ON `session` (`workspace_id`);--> statement-breakpoint
CREATE INDEX `session_parent_idx` ON `session` (`parent_id`);--> statement-breakpoint
PRAGMA foreign_keys=ON;
SELECT (SELECT COUNT(*) FROM `session`) AS `post_rows_session`;
SELECT (SELECT COUNT(*) FROM `__backup_20260509204500_session_fk_constraints_session`) AS `backup_rows_session`;
DROP TABLE `__backup_20260509204500_session_fk_constraints_session`;
