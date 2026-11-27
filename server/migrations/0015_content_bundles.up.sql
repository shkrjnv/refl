-- The complete serialized bundle and its SHA-256, stored with the content
-- version so `GET /v1/content/active` can serve exactly what was imported.
-- NULL only for versions imported before this migration.
ALTER TABLE content_versions ADD COLUMN bundle jsonb;
ALTER TABLE content_versions ADD COLUMN bundle_digest text
    CHECK (bundle_digest ~ '^[0-9a-f]{64}$');
