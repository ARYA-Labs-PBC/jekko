-- Rollback: ALTER TABLE `event_sequence` DROP COLUMN `owner_id`;
-- Pre-flight: SELECT COUNT(*) FROM `event_sequence`;
ALTER TABLE `event_sequence` ADD `owner_id` text;