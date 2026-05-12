-- Constraint: every part row must reference an existing session.
SELECT p.`id`
FROM `part` AS p
LEFT JOIN `session` AS s ON s.`id` = p.`session_id`
WHERE s.`id` IS NULL
LIMIT 1;
