-- Enforce that owner_id, when present, is a non-empty token.
-- rollback: drop the owner_id guard triggers.
CREATE TRIGGER IF NOT EXISTS `event_sequence_owner_id_guard_insert`
BEFORE INSERT ON `event_sequence`
FOR EACH ROW
WHEN NEW.`owner_id` IS NOT NULL AND trim(NEW.`owner_id`) = ''
BEGIN
  SELECT RAISE(ABORT, 'event_sequence.owner_id must be NULL or a non-empty token');
END;
--> statement-breakpoint
CREATE TRIGGER IF NOT EXISTS `event_sequence_owner_id_guard_update`
BEFORE UPDATE OF `owner_id` ON `event_sequence`
FOR EACH ROW
WHEN NEW.`owner_id` IS NOT NULL AND trim(NEW.`owner_id`) = ''
BEGIN
  SELECT RAISE(ABORT, 'event_sequence.owner_id must be NULL or a non-empty token');
END;
