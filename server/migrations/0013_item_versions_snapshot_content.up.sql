-- Model projections of every published item version of a content version
-- (docs/02, «Идентификаторы и версии»): the fold resolves (item_id, version)
-- of old events through them.
CREATE TABLE content_item_versions (
    content_version text NOT NULL REFERENCES content_versions (version) ON DELETE CASCADE,
    item_id text NOT NULL,
    version integer NOT NULL CHECK (version >= 0),
    projection jsonb NOT NULL,
    PRIMARY KEY (content_version, item_id, version)
);

GRANT SELECT, INSERT, UPDATE, DELETE ON content_item_versions TO refl_app;

-- A snapshot is valid for the content version it was computed with
-- (docs/02, «Событие и лог»): another content version means a full refold,
-- which overwrites the row. Snapshots are derived data, so the ones taken
-- before this column existed are dropped and rebuilt from the log.
DELETE FROM model_snapshots;
ALTER TABLE model_snapshots ADD COLUMN content_version text NOT NULL;
