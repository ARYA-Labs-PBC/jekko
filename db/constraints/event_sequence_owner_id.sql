-- Constraint: event_sequence.owner_id is either NULL or a non-empty owner token.
-- This is a lightweight verification query for the db constraint lane.
SELECT aggregate_id
FROM event_sequence
WHERE owner_id IS NOT NULL
  AND trim(owner_id) = '';
