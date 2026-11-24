DELETE FROM model_snapshots;
ALTER TABLE model_snapshots RENAME COLUMN fold_version TO content_version;
ALTER TABLE content_versions DROP COLUMN fold_version;
