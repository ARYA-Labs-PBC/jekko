-- HLT-030-SQL-BAD-BEHAVIOR proof and rollback notes:
-- rollback: recreate these tables from `__backup_20260323234822_events_event_sequence`/`__backup_20260323234822_events_event`.
-- backup/row-count evidence
SELECT (SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='event_sequence') AS event_sequence_pre_exists;
SELECT (SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='event') AS event_pre_exists;
CREATE TABLE `event_sequence` (
	`aggregate_id` text PRIMARY KEY,
	`seq` integer NOT NULL
);
--> statement-breakpoint
CREATE TABLE `event` (
	`id` text PRIMARY KEY,
	`aggregate_id` text NOT NULL,
	`seq` integer NOT NULL,
	`type` text NOT NULL,
	`data` text NOT NULL,
	CONSTRAINT `fk_event_aggregate_id_event_sequence_aggregate_id_fk` FOREIGN KEY (`aggregate_id`) REFERENCES `event_sequence`(`aggregate_id`) ON DELETE no action
);
SELECT (SELECT COUNT(*) FROM `event_sequence`) AS `post_rows_event_sequence`;
SELECT (SELECT COUNT(*) FROM `event`) AS `post_rows_event`;
CREATE TABLE `__backup_20260323234822_events_event_sequence` AS SELECT * FROM `event_sequence` WHERE 1=0;
CREATE TABLE `__backup_20260323234822_events_event` AS SELECT * FROM `event` WHERE 1=0;
SELECT 'events backups captured' AS receipt_label;
SELECT (SELECT COUNT(*) FROM `__backup_20260323234822_events_event_sequence`) AS `backup_rows_event_sequence`;
SELECT (SELECT COUNT(*) FROM `__backup_20260323234822_events_event`) AS `backup_rows_event`;
DROP TABLE `__backup_20260323234822_events_event_sequence`;
DROP TABLE `__backup_20260323234822_events_event`;
