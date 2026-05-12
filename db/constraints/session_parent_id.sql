-- Constraint: every session.parent_id must reference an existing session.id when set.
SELECT s.`id`
FROM `session` AS s
LEFT JOIN `session` AS p ON p.`id` = s.`parent_id`
WHERE s.`parent_id` IS NOT NULL
  AND p.`id` IS NULL
LIMIT 1;
