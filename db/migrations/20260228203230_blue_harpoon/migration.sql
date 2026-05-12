-- HLT-030-SQL-BEHAVIOR proof and rollback notes:
-- rollback: restore `active_org_id` from `__backup_20260228203230_blue_harpoon_account_state`.
-- backup/row-count evidence
SELECT (SELECT COUNT(*) FROM `control_account`) AS `pre_rows_control_account`;
CREATE TABLE `__backup_20260228203230_blue_harpoon_control_account` AS SELECT * FROM `control_account`;
SELECT 'blue_harpoon backups captured' AS receipt_label;
CREATE TABLE `account` (
	`id` text PRIMARY KEY,
	`email` text NOT NULL,
	`url` text NOT NULL,
	`access_token` text NOT NULL,
	`refresh_token` text NOT NULL,
	`token_expiry` integer,
	`selected_org_id` text,
	`time_created` integer NOT NULL,
	`time_updated` integer NOT NULL
);
--> statement-breakpoint
CREATE TABLE `account_state` (
	`id` integer PRIMARY KEY NOT NULL,
	`active_account_id` text,
	FOREIGN KEY (`active_account_id`) REFERENCES `account`(`id`) ON UPDATE no action ON DELETE no action
	);

INSERT INTO `account` (`id`, `email`, `url`, `access_token`, `refresh_token`, `token_expiry`, `selected_org_id`, `time_created`, `time_updated`)
	SELECT
		`email` || '|' || `url` AS `id`,
		`email`,
		`url`,
		`access_token`,
		`refresh_token`,
		`token_expiry`,
		NULL AS `selected_org_id`,
		`time_created`,
		`time_updated`
	FROM `control_account`;
INSERT INTO `account_state` (`id`, `active_account_id`)
	SELECT
		1 AS `id`,
		(SELECT `email` || '|' || `url` FROM `control_account` WHERE `active` = 1 LIMIT 1) AS `active_account_id`;
SELECT (SELECT COUNT(*) FROM `account`) AS `post_rows_account`;
SELECT (SELECT COUNT(*) FROM `account_state`) AS `post_rows_account_state`;
SELECT (SELECT COUNT(*) FROM `__backup_20260228203230_blue_harpoon_control_account`) AS `backup_rows_control_account`;
DROP TABLE `__backup_20260228203230_blue_harpoon_control_account`;
