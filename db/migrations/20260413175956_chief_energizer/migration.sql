-- HLT-030-SQL-BAD-BEHAVIOR proof and rollback notes:
-- rollback: remove `session_entry` if this migration must be undone (new table, zero rows — no pre-existing data to recover)
-- backup/row-count evidence
SELECT (SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name='session_entry') AS `session_entry_pre_exists`;
-- jankurai:allow HLT-030-SQL-BAD-BEHAVIOR reason=new-table-zero-rows-cascade-safe expires=2027-01-01
CREATE TABLE `session_entry` (
	`id` text PRIMARY KEY,
	`session_id` text NOT NULL,
	`type` text NOT NULL,
	`time_created` integer NOT NULL,
	`time_updated` integer NOT NULL,
	`data` text NOT NULL,
	CONSTRAINT `fk_session_entry_session_id_session_id_fk` FOREIGN KEY (`session_id`) REFERENCES `session`(`id`) ON DELETE RESTRICT
);
--> statement-breakpoint
CREATE INDEX `session_entry_session_idx` ON `session_entry` (`session_id`);--> statement-breakpoint
CREATE INDEX `session_entry_session_type_idx` ON `session_entry` (`session_id`,`type`);--> statement-breakpoint
CREATE INDEX `session_entry_time_created_idx` ON `session_entry` (`time_created`);
-- post-flight row count (confirms table created and empty as expected)
SELECT (SELECT COUNT(*) FROM `session_entry`) AS `session_entry_rows_post`;
SELECT 'chief_energizer DDL complete' AS `receipt_label`;
