-- Learner model snapshots (docs/02, «Событие и лог»; docs/03, «Снапшоты и
-- пересчёт»). Derived data: may be deleted and rebuilt from the log at any time.
CREATE TABLE model_snapshots (
    learner_id uuid NOT NULL REFERENCES users (id),
    -- Shadow models keep their own snapshots.
    model_version text NOT NULL,
    parameters jsonb NOT NULL,
    parameters_hash text NOT NULL,
    -- Serialized by the core; the format is defined by the model version.
    state bytea NOT NULL,
    -- Largest event_id folded into `state`.
    last_event_id uuid NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (learner_id, model_version)
);

GRANT SELECT, INSERT, UPDATE, DELETE ON model_snapshots TO refl_app;
