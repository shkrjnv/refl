-- Subjects of a content version, and each KC's level as a rank in its
-- subject's scale (docs/10, «Предмет», «Шкала уровней»).
CREATE TABLE content_subjects (
    content_version text NOT NULL REFERENCES content_versions (version) ON DELETE CASCADE,
    id text NOT NULL,
    body jsonb NOT NULL,
    PRIMARY KEY (content_version, id)
);

-- NULL only for versions imported from bundles before format 3, which had
-- no subjects.
ALTER TABLE content_kcs ADD COLUMN level_rank integer CHECK (level_rank >= 0);

GRANT SELECT, INSERT, UPDATE, DELETE ON content_subjects TO refl_app;
