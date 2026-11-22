-- Content bundles in Postgres (docs/02, «Идентификаторы и версии»): per-entity
-- tables keyed by content version, the entity as JSONB plus indexed keys.
-- Exactly one version is active; old versions stay so that the
-- (item_id, version) of old events always resolve.
CREATE TABLE content_versions (
    version text PRIMARY KEY CHECK (version ~ '^[0-9a-f]{64}$'),
    format_version integer NOT NULL,
    normalization_version integer NOT NULL,
    -- Token alphabet of the version's automata.
    tokens jsonb NOT NULL,
    imported_at timestamptz NOT NULL DEFAULT now(),
    active boolean NOT NULL DEFAULT false
);
CREATE UNIQUE INDEX content_versions_one_active ON content_versions ((true)) WHERE active;

CREATE TABLE content_kcs (
    content_version text NOT NULL REFERENCES content_versions (version) ON DELETE CASCADE,
    id text NOT NULL,
    status text NOT NULL CHECK (status IN ('draft', 'live')),
    body jsonb NOT NULL,
    PRIMARY KEY (content_version, id)
);

CREATE TABLE content_edges (
    content_version text NOT NULL REFERENCES content_versions (version) ON DELETE CASCADE,
    kind text NOT NULL,
    from_kc text NOT NULL,
    to_kc text NOT NULL,
    PRIMARY KEY (content_version, kind, from_kc, to_kc)
);

CREATE TABLE content_requires_closure (
    content_version text NOT NULL REFERENCES content_versions (version) ON DELETE CASCADE,
    kc text NOT NULL,
    required_kc text NOT NULL,
    PRIMARY KEY (content_version, kc, required_kc)
);

CREATE TABLE content_misconceptions (
    content_version text NOT NULL REFERENCES content_versions (version) ON DELETE CASCADE,
    id text NOT NULL,
    body jsonb NOT NULL,
    PRIMARY KEY (content_version, id)
);

CREATE TABLE content_explanations (
    content_version text NOT NULL REFERENCES content_versions (version) ON DELETE CASCADE,
    id text NOT NULL,
    target_kind text NOT NULL,
    target_id text NOT NULL,
    depth text NOT NULL,
    lang text NOT NULL,
    body jsonb NOT NULL,
    PRIMARY KEY (content_version, id)
);
-- Lookup by key, e.g. a misconception's short explanation in a language.
CREATE INDEX content_explanations_key
    ON content_explanations (content_version, target_kind, target_id, depth, lang);

CREATE TABLE content_instructions (
    content_version text NOT NULL REFERENCES content_versions (version) ON DELETE CASCADE,
    id text NOT NULL,
    body jsonb NOT NULL,
    PRIMARY KEY (content_version, id)
);

CREATE TABLE content_lessons (
    content_version text NOT NULL REFERENCES content_versions (version) ON DELETE CASCADE,
    id text NOT NULL,
    body jsonb NOT NULL,
    PRIMARY KEY (content_version, id)
);

CREATE TABLE content_items (
    content_version text NOT NULL REFERENCES content_versions (version) ON DELETE CASCADE,
    id text NOT NULL,
    version integer NOT NULL CHECK (version >= 0),
    status text NOT NULL CHECK (status IN ('draft', 'review', 'live', 'retired')),
    session_eligible boolean NOT NULL,
    body jsonb NOT NULL,
    -- Compiled automata per slot.
    slots jsonb NOT NULL,
    PRIMARY KEY (content_version, id)
);
-- Events reference (item_id, version).
CREATE INDEX content_items_ref ON content_items (id, version);

CREATE TABLE content_items_by_kc (
    content_version text NOT NULL REFERENCES content_versions (version) ON DELETE CASCADE,
    kc text NOT NULL,
    role text NOT NULL CHECK (role IN ('primary', 'secondary')),
    item_id text NOT NULL,
    item_version integer NOT NULL,
    PRIMARY KEY (content_version, kc, role, item_id)
);

GRANT SELECT, INSERT, UPDATE, DELETE
    ON content_versions, content_kcs, content_edges, content_requires_closure,
       content_misconceptions, content_explanations, content_instructions,
       content_lessons, content_items, content_items_by_kc
    TO refl_app;
