-- Rollback: ALTER TABLE `session` DROP COLUMN `path`;
-- Pre-flight: SELECT COUNT(*) FROM `session`;
ALTER TABLE `session` ADD `path` text;