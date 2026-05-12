-- Rollback: ALTER TABLE `pending` RENAME TO `todo`;
-- Pre-flight: SELECT (SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name='todo') AS pre_exists;
ALTER TABLE `todo` RENAME TO `pending`;
