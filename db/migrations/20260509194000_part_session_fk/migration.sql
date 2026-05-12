-- Enforce part.session_id as durable truth so part rows cannot orphan their session.
-- rollback: rename `__backup_20260509194000_part_session_fk_part` back to `part` if recovery is needed.
SELECT (SELECT COUNT(*) FROM `part`) AS `pre_rows_part`;
PRAGMA foreign_keys=OFF;--> statement-breakpoint
ALTER TABLE `part` RENAME TO `__backup_20260509194000_part_session_fk_part`;--> statement-breakpoint
CREATE TABLE `part` (
	`id` text PRIMARY KEY,
	`message_id` text NOT NULL,
	`session_id` text NOT NULL,
	`time_created` integer NOT NULL,
	`time_updated` integer NOT NULL,
	`data` text NOT NULL,
	CONSTRAINT `fk_part_message_id_message_id_fk` FOREIGN KEY (`message_id`) REFERENCES `message`(`id`) ON DELETE CASCADE,
	CONSTRAINT `fk_part_session_id_session_id_fk` FOREIGN KEY (`session_id`) REFERENCES `session`(`id`) ON DELETE CASCADE
);--> statement-breakpoint
INSERT INTO `part`(`id`, `message_id`, `session_id`, `time_created`, `time_updated`, `data`) SELECT `id`, `message_id`, `session_id`, `time_created`, `time_updated`, `data` FROM `__backup_20260509194000_part_session_fk_part`;--> statement-breakpoint
DROP INDEX IF EXISTS `part_message_id_id_idx`;--> statement-breakpoint
DROP INDEX IF EXISTS `part_session_idx`;--> statement-breakpoint
CREATE INDEX `part_message_id_id_idx` ON `part` (`message_id`,`id`);--> statement-breakpoint
CREATE INDEX `part_session_idx` ON `part` (`session_id`);--> statement-breakpoint
PRAGMA foreign_keys=ON;
SELECT (SELECT COUNT(*) FROM `part`) AS `post_rows_part`;
SELECT (SELECT COUNT(*) FROM `__backup_20260509194000_part_session_fk_part`) AS `backup_rows_part`;
DROP TABLE `__backup_20260509194000_part_session_fk_part`;
