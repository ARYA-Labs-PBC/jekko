-- Rollback: DROP INDEX `session_workspace_idx`; ALTER TABLE `session` DROP COLUMN `workspace_id`;
-- Pre-flight: SELECT COUNT(*) FROM `session`;
ALTER TABLE `session` ADD `workspace_id` text;--> statement-breakpoint
CREATE INDEX `session_workspace_idx` ON `session` (`workspace_id`);