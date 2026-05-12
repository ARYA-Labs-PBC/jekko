-- Rollback: ALTER TABLE `project` DROP COLUMN `icon_url_override`;
-- Pre-flight: SELECT COUNT(*) FROM `project` WHERE `icon_url` IS NOT NULL;
ALTER TABLE `project` ADD `icon_url_override` text;
UPDATE `project` SET `icon_url_override` = `icon_url` WHERE `icon_url` IS NOT NULL;
