-- HLT-030-SQL-BAD-BEHAVIOR proof and rollback notes:
-- rollback: drop `workspace` table entirely from this migration.
-- backup/row-count evidence
SELECT (SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name='workspace') AS workspace_pre_exists;
CREATE TABLE `workspace` (
	`id` text PRIMARY KEY,
	`branch` text,
	`project_id` text NOT NULL,
	`config` text NOT NULL,
    CONSTRAINT `fk_workspace_project_id_project_id_fk` FOREIGN KEY (`project_id`) REFERENCES `project`(`id`) ON DELETE CASCADE
);
SELECT (SELECT COUNT(*) FROM `workspace`) AS `workspace_rows_post`;
CREATE TABLE `__backup_20260225215848_workspace_workspace` AS SELECT * FROM `workspace` WHERE 1=0;
SELECT (SELECT COUNT(*) FROM `__backup_20260225215848_workspace_workspace`) AS `workspace_backup_rows`;
DROP TABLE `__backup_20260225215848_workspace_workspace`;
