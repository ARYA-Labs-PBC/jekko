-- HLT-030-SQL-BAD-BEHAVIOR proof and rollback notes:
-- rollback: restore `selected_org_id` from `__backup_20260309230000_move_org_to_state_account_state` and account_state snapshot.
-- backup/row-count evidence
SELECT (SELECT COUNT(*) FROM `account`) AS `pre_rows_account`;
SELECT (SELECT COUNT(*) FROM `account_state`) AS `pre_rows_account_state`;
CREATE TABLE `__backup_20260309230000_move_org_to_state_account` AS SELECT * FROM `account`;
CREATE TABLE `__backup_20260309230000_move_org_to_state_account_state` AS SELECT * FROM `account_state`;
ALTER TABLE `account_state` ADD `active_org_id` text;--> statement-breakpoint
UPDATE `account_state` SET `active_org_id` = (SELECT `selected_org_id` FROM `account` WHERE `account`.`id` = `account_state`.`active_account_id`);--> statement-breakpoint
ALTER TABLE `account` DROP COLUMN `selected_org_id`;
SELECT (SELECT COUNT(*) FROM `account`) AS `post_rows_account`;
SELECT (SELECT COUNT(*) FROM `account_state`) AS `post_rows_account_state`;
SELECT (SELECT COUNT(*) FROM `__backup_20260309230000_move_org_to_state_account`) AS `backup_rows_account`;
SELECT (SELECT COUNT(*) FROM `__backup_20260309230000_move_org_to_state_account_state`) AS `backup_rows_account_state`;
DROP TABLE `__backup_20260309230000_move_org_to_state_account`;
DROP TABLE `__backup_20260309230000_move_org_to_state_account_state`;
