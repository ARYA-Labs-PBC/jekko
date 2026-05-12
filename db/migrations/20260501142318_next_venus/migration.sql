-- Rollback: ALTER TABLE `session` DROP COLUMN `agent`; ALTER TABLE `session` DROP COLUMN `model`;
-- Pre-flight: SELECT COUNT(*) FROM `session`;
ALTER TABLE `session` ADD `agent` text;--> statement-breakpoint
ALTER TABLE `session` ADD `model` text;