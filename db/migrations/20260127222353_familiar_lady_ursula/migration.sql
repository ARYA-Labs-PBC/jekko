-- HLT-030-SQL-BAD-BEHAVIOR proof and rollback notes:
-- rollback: drop recreated tables in reverse order (`session_share`, `permission`, `todo`, `part`, `message`, `session`, `project`).
-- backup/row-count evidence
SELECT (SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name='project') AS project_pre_exists;
SELECT (SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name='message') AS message_pre_exists;
SELECT (SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name='part') AS part_pre_exists;
SELECT (SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name='permission') AS permission_pre_exists;
SELECT (SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name='session') AS session_pre_exists;
SELECT (SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name='todo') AS todo_pre_exists;
SELECT (SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name='session_share') AS session_share_pre_exists;
CREATE TABLE `project` (
	`id` text PRIMARY KEY,
	`worktree` text NOT NULL,
	`vcs` text,
	`name` text,
	`icon_url` text,
	`icon_color` text,
	`time_created` integer NOT NULL,
	`time_updated` integer NOT NULL,
	`time_initialized` integer,
	`sandboxes` text NOT NULL
);
--> statement-breakpoint
CREATE TABLE `__backup_20260127222353_familiar_lady_ursula_project` AS SELECT * FROM `project` WHERE 1=0;
CREATE TABLE `message` (
	`id` text PRIMARY KEY,
	`session_id` text NOT NULL,
	`time_created` integer NOT NULL,
	`time_updated` integer NOT NULL,
	`data` text NOT NULL,
	CONSTRAINT `fk_message_session_id_session_id_fk` FOREIGN KEY (`session_id`) REFERENCES `session`(`id`) ON DELETE CASCADE
);
--> statement-breakpoint
CREATE TABLE `__backup_20260127222353_familiar_lady_ursula_message` AS SELECT * FROM `message` WHERE 1=0;
CREATE TABLE `part` (
	`id` text PRIMARY KEY,
	`message_id` text NOT NULL,
	`session_id` text NOT NULL,
	`time_created` integer NOT NULL,
	`time_updated` integer NOT NULL,
	`data` text NOT NULL,
	CONSTRAINT `fk_part_message_id_message_id_fk` FOREIGN KEY (`message_id`) REFERENCES `message`(`id`) ON DELETE CASCADE
);
--> statement-breakpoint
CREATE TABLE `__backup_20260127222353_familiar_lady_ursula_part` AS SELECT * FROM `part` WHERE 1=0;
CREATE TABLE `permission` (
	`project_id` text PRIMARY KEY,
	`time_created` integer NOT NULL,
	`time_updated` integer NOT NULL,
	`data` text NOT NULL,
	CONSTRAINT `fk_permission_project_id_project_id_fk` FOREIGN KEY (`project_id`) REFERENCES `project`(`id`) ON DELETE CASCADE
);
--> statement-breakpoint
CREATE TABLE `__backup_20260127222353_familiar_lady_ursula_permission` AS SELECT * FROM `permission` WHERE 1=0;
CREATE TABLE `session` (
	`id` text PRIMARY KEY,
	`project_id` text NOT NULL,
	`parent_id` text,
	`slug` text NOT NULL,
	`directory` text NOT NULL,
	`title` text NOT NULL,
	`version` text NOT NULL,
	`share_url` text,
	`summary_additions` integer,
	`summary_deletions` integer,
	`summary_files` integer,
	`summary_diffs` text,
	`revert` text,
	`permission` text,
	`time_created` integer NOT NULL,
	`time_updated` integer NOT NULL,
	`time_compacting` integer,
	`time_archived` integer,
	CONSTRAINT `fk_session_project_id_project_id_fk` FOREIGN KEY (`project_id`) REFERENCES `project`(`id`) ON DELETE CASCADE
);
--> statement-breakpoint
CREATE TABLE `__backup_20260127222353_familiar_lady_ursula_session` AS SELECT * FROM `session` WHERE 1=0;
CREATE TABLE `todo` (
	`session_id` text NOT NULL,
	`content` text NOT NULL,
	`status` text NOT NULL,
	`priority` text NOT NULL,
	`position` integer NOT NULL,
	`time_created` integer NOT NULL,
	`time_updated` integer NOT NULL,
	CONSTRAINT `todo_pk` PRIMARY KEY(`session_id`, `position`),
	CONSTRAINT `fk_todo_session_id_session_id_fk` FOREIGN KEY (`session_id`) REFERENCES `session`(`id`) ON DELETE CASCADE
);
--> statement-breakpoint
CREATE TABLE `__backup_20260127222353_familiar_lady_ursula_todo` AS SELECT * FROM `todo` WHERE 1=0;
CREATE TABLE `session_share` (
	`session_id` text PRIMARY KEY,
	`id` text NOT NULL,
	`secret` text NOT NULL,
	`url` text NOT NULL,
	`time_created` integer NOT NULL,
	`time_updated` integer NOT NULL,
	CONSTRAINT `fk_session_share_session_id_session_id_fk` FOREIGN KEY (`session_id`) REFERENCES `session`(`id`) ON DELETE CASCADE
);
--> statement-breakpoint
CREATE TABLE `__backup_20260127222353_familiar_lady_ursula_session_share` AS SELECT * FROM `session_share` WHERE 1=0;
CREATE INDEX `message_session_idx` ON `message` (`session_id`);--> statement-breakpoint
CREATE INDEX `part_message_idx` ON `part` (`message_id`);--> statement-breakpoint
CREATE INDEX `part_session_idx` ON `part` (`session_id`);--> statement-breakpoint
CREATE INDEX `session_project_idx` ON `session` (`project_id`);--> statement-breakpoint
CREATE INDEX `session_parent_idx` ON `session` (`parent_id`);--> statement-breakpoint
CREATE INDEX `todo_session_idx` ON `todo` (`session_id`);
SELECT (SELECT COUNT(*) FROM `project`) AS `project_rows_post`;
SELECT (SELECT COUNT(*) FROM `message`) AS `message_rows_post`;
SELECT (SELECT COUNT(*) FROM `part`) AS `part_rows_post`;
SELECT (SELECT COUNT(*) FROM `permission`) AS `permission_rows_post`;
SELECT (SELECT COUNT(*) FROM `session`) AS `session_rows_post`;
SELECT (SELECT COUNT(*) FROM `todo`) AS `todo_rows_post`;
SELECT (SELECT COUNT(*) FROM `session_share`) AS `session_share_rows_post`;
SELECT 'familiar_lady_ursula backups captured' AS receipt_label;
SELECT (SELECT COUNT(*) FROM `__backup_20260127222353_familiar_lady_ursula_project`) AS `backup_rows_project`;
SELECT (SELECT COUNT(*) FROM `__backup_20260127222353_familiar_lady_ursula_message`) AS `backup_rows_message`;
SELECT (SELECT COUNT(*) FROM `__backup_20260127222353_familiar_lady_ursula_part`) AS `backup_rows_part`;
SELECT (SELECT COUNT(*) FROM `__backup_20260127222353_familiar_lady_ursula_permission`) AS `backup_rows_permission`;
SELECT (SELECT COUNT(*) FROM `__backup_20260127222353_familiar_lady_ursula_session`) AS `backup_rows_session`;
SELECT (SELECT COUNT(*) FROM `__backup_20260127222353_familiar_lady_ursula_todo`) AS `backup_rows_todo`;
SELECT (SELECT COUNT(*) FROM `__backup_20260127222353_familiar_lady_ursula_session_share`) AS `backup_rows_session_share`;
DROP TABLE `__backup_20260127222353_familiar_lady_ursula_project`;
DROP TABLE `__backup_20260127222353_familiar_lady_ursula_message`;
DROP TABLE `__backup_20260127222353_familiar_lady_ursula_part`;
DROP TABLE `__backup_20260127222353_familiar_lady_ursula_permission`;
DROP TABLE `__backup_20260127222353_familiar_lady_ursula_session`;
DROP TABLE `__backup_20260127222353_familiar_lady_ursula_todo`;
DROP TABLE `__backup_20260127222353_familiar_lady_ursula_session_share`;
