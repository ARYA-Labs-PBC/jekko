-- HLT-030-SQL-BAD-BEHAVIOR proof and rollback notes:
-- rollback: recreate dropped indexes from `__backup_20260312043431_session_message_cursor_message`.
-- backup/row-count evidence
SELECT (SELECT COUNT(*) FROM `message`) AS `pre_rows_message`;
SELECT (SELECT COUNT(*) FROM `part`) AS `pre_rows_part`;
CREATE TABLE `__backup_20260312043431_session_message_cursor_message` AS SELECT * FROM `message`;
CREATE TABLE `__backup_20260312043431_session_message_cursor_part` AS SELECT * FROM `part`;
DROP INDEX IF EXISTS `message_session_idx`;--> statement-breakpoint
DROP INDEX IF EXISTS `part_message_idx`;--> statement-breakpoint
CREATE INDEX `message_session_time_created_id_idx` ON `message` (`session_id`,`time_created`,`id`);--> statement-breakpoint
CREATE INDEX `part_message_id_id_idx` ON `part` (`message_id`,`id`);
SELECT (SELECT COUNT(*) FROM `message`) AS `post_rows_message`;
SELECT (SELECT COUNT(*) FROM `part`) AS `post_rows_part`;
SELECT (SELECT COUNT(*) FROM `__backup_20260312043431_session_message_cursor_message`) AS `backup_rows_message`;
SELECT (SELECT COUNT(*) FROM `__backup_20260312043431_session_message_cursor_part`) AS `backup_rows_part`;
DROP TABLE `__backup_20260312043431_session_message_cursor_message`;
DROP TABLE `__backup_20260312043431_session_message_cursor_part`;
