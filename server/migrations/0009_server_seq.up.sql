-- Receipt order for sync (docs/13, «Протокол синхронизации»): event_id is
-- the fold order, server_seq the order in which the server stored events.
-- A client's cursor is the last server_seq it has received. Values are
-- strictly increasing but not contiguous: an insert ignored as a duplicate
-- event_id still consumes one.
ALTER TABLE events ADD COLUMN server_seq bigint GENERATED ALWAYS AS IDENTITY;

CREATE INDEX events_learner_seq ON events (learner_id, server_seq);
