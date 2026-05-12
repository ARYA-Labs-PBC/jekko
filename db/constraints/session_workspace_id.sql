-- Constraint: every session.workspace_id must reference an existing workspace.id when set.
SELECT s.`id`
FROM `session` AS s
LEFT JOIN `workspace` AS w ON w.`id` = s.`workspace_id`
WHERE s.`workspace_id` IS NOT NULL
  AND w.`id` IS NULL
LIMIT 1;
