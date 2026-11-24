-- The checker's lexicon (docs/02, «Словарь проверщика»): a separate file per
-- version, stored once however many content versions reference it.
CREATE TABLE content_lexicons (
    version text PRIMARY KEY CHECK (version ~ '^[0-9a-f]{64}$'),
    words integer NOT NULL CHECK (words >= 0),
    -- The file: lowercase words, sorted, one per line.
    body text NOT NULL,
    imported_at timestamptz NOT NULL DEFAULT now()
);

-- NULL only for versions imported from bundles before format 4, which had
-- no lexicon.
ALTER TABLE content_versions
    ADD COLUMN lexicon_version text REFERENCES content_lexicons (version);

GRANT SELECT, INSERT ON content_lexicons TO refl_app;
