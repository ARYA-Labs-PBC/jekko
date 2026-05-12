-- Rollback: ALTER TABLE `project` DROP COLUMN `commands`;
-- Pre-flight: SELECT COUNT(*) FROM `project` WHERE `commands` IS NOT NULL;
ALTER TABLE `project` ADD `commands` text;