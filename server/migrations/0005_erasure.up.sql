-- Account erasure, the single exception to the append-only log (docs/02,
-- «Событие и лог»): one function deletes all events and snapshots of one
-- learner and records the erasure without personal data. It runs as the
-- schema owner; only refl_erasure may execute it.
CREATE TABLE erasures (
    learner_id uuid PRIMARY KEY,
    erased_at timestamptz NOT NULL DEFAULT now()
);

CREATE FUNCTION erase_learner(learner uuid) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER SET search_path FROM CURRENT AS $$
BEGIN
    PERFORM set_config('refl.erasing_learner', learner::text, true);
    DELETE FROM model_snapshots WHERE learner_id = learner;
    DELETE FROM events WHERE learner_id = learner;
    PERFORM set_config('refl.erasing_learner', '', true);
    INSERT INTO erasures (learner_id) VALUES (learner) ON CONFLICT DO NOTHING;
END $$;

REVOKE ALL ON FUNCTION erase_learner(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION erase_learner(uuid) TO refl_erasure;
GRANT SELECT ON erasures TO refl_app;
