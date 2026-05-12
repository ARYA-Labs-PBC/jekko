-- HLT-030-SQL-BAD-BEHAVIOR proof and rollback notes:
-- rollback: restore dropped `config` via `__backup_20260303231226_add_workspace_fields_workspace`.
-- backup/row-count evidence
SELECT (SELECT COUNT(*) FROM `workspace`) AS `pre_rows_workspace`;
CREATE TABLE `__backup_20260303231226_add_workspace_fields_workspace` AS SELECT * FROM `workspace`;
ALTER TABLE `workspace` ADD `type` text NOT NULL;--> statement-breakpoint
ALTER TABLE `workspace` ADD `name` text;--> statement-breakpoint
ALTER TABLE `workspace` ADD `directory` text;--> statement-breakpoint
ALTER TABLE `workspace` ADD `extra` text;--> statement-breakpoint
ALTER TABLE `workspace` DROP COLUMN `config`;
SELECT (SELECT COUNT(*) FROM `workspace`) AS `post_rows_workspace`;
SELECT (SELECT COUNT(*) FROM `__backup_20260303231226_add_workspace_fields_workspace`) AS `backup_rows_workspace`;
DROP TABLE `__backup_20260303231226_add_workspace_fields_workspace`;
