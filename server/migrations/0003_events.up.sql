-- The event log (docs/02, «Событие и лог»; docs/07, «Модель разблокировок»).
-- Append-only: the database rejects UPDATE, DELETE and TRUNCATE; the
-- application role may only SELECT and INSERT. Events are validated by the
-- core before insertion and stored as sent; envelope fields are columns and
-- the payload is JSONB, so older schema versions are upcast on read.
CREATE TABLE events (
    event_id uuid PRIMARY KEY CHECK (substr(event_id::text, 15, 1) = '7'),
    learner_id uuid NOT NULL REFERENCES users (id),
    session_id uuid,
    schema_version smallint NOT NULL CHECK (schema_version > 0),
    -- Milliseconds since the Unix epoch, exactly as in the core.
    client_ts bigint NOT NULL,
    server_ts bigint NOT NULL,
    -- The payload's `type` tag.
    event_type text NOT NULL,
    payload jsonb NOT NULL CHECK (payload ->> 'type' = event_type),
    -- Uniqueness key of server-derived events; NULL for all others.
    derived_kind text,
    derived_key text,
    CHECK ((derived_kind IS NULL) = (derived_key IS NULL))
);

-- Fold order is event_id (UUIDv7) per learner.
CREATE INDEX events_learner_order ON events (learner_id, event_id);

-- At most one derived event per (learner, event type, kind, key).
CREATE UNIQUE INDEX events_derived_unique
    ON events (learner_id, event_type, derived_kind, derived_key)
    WHERE derived_key IS NOT NULL;

-- Only erase_learner() may delete, and only the learner it is erasing.
CREATE FUNCTION events_append_only() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE'
        AND current_setting('refl.erasing_learner', true) = OLD.learner_id::text THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION 'events are append-only: % rejected', TG_OP
        USING ERRCODE = 'restrict_violation';
END $$;

CREATE TRIGGER events_append_only BEFORE UPDATE OR DELETE ON events
    FOR EACH ROW EXECUTE FUNCTION events_append_only();
CREATE TRIGGER events_no_truncate BEFORE TRUNCATE ON events
    FOR EACH STATEMENT EXECUTE FUNCTION events_append_only();

GRANT SELECT, INSERT ON events TO refl_app;
