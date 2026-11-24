-- The version of everything the fold reads from a bundle except item
-- projections (docs/02, «Версия свёртки»): computed by the core at import.
-- NULL only for versions imported before this column existed.
ALTER TABLE content_versions ADD COLUMN fold_version text;

-- Snapshots key on the fold version, not on the bundle: a bundle with the
-- same fold version gives the same state. Snapshots are derived data, so
-- the ones keyed by a content version are dropped and rebuilt from the log.
DELETE FROM model_snapshots;
ALTER TABLE model_snapshots RENAME COLUMN content_version TO fold_version;
