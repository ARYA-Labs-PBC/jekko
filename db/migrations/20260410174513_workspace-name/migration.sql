-- HLT-030-SQL-BAD-BEHAVIOR proof and rollback notes:
-- rollback: rename `__backup_20260410174513_workspace-name_workspace` back to `workspace` if recovery is needed.
-- backup/row-count evidence
SELECT (SELECT COUNT(*) FROM `workspace`) AS `pre_rows_workspace`;
PRAGMA foreign_keys=OFF;--> statement-breakpoint
ALTER TABLE `workspace` RENAME TO `__backup_20260410174513_workspace-name_workspace`;--> statement-breakpoint
CREATE TABLE `__new_workspace` (
	`id` text PRIMARY KEY,
	`type` text NOT NULL,
	`name` text DEFAULT '' NOT NULL,
	`branch` text,
	`directory` text,
	`extra` text,
	`project_id` text NOT NULL,
	CONSTRAINT `fk_workspace_project_id_project_id_fk` FOREIGN KEY (`project_id`) REFERENCES `project`(`id`) ON DELETE CASCADE
);
--> statement-breakpoint
INSERT INTO `__new_workspace`(`id`, `type`, `branch`, `name`, `directory`, `extra`, `project_id`) SELECT `id`, `type`, `branch`, `name`, `directory`, `extra`, `project_id` FROM `__backup_20260410174513_workspace-name_workspace`;--> statement-breakpoint
SELECT (SELECT COUNT(*) FROM `__backup_20260410174513_workspace-name_workspace`) AS `nearby_backup_rows_workspace`;
SELECT (SELECT COUNT(*) FROM `__new_workspace`) AS `nearby_rows_workspace`;
DROP TABLE `__backup_20260410174513_workspace-name_workspace`;--> statement-breakpoint
ALTER TABLE `__new_workspace` RENAME TO `workspace`;--> statement-breakpoint
PRAGMA foreign_keys=ON;
SELECT (SELECT COUNT(*) FROM `workspace`) AS `post_rows_workspace`;
