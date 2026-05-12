-- HLT-030-SQL-BAD-BEHAVIOR proof and rollback notes:
-- rollback: rename `session_message` back to `session_entry` if needed.
ALTER TABLE `session_entry` RENAME TO `session_message`;
