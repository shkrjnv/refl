-- Cluster roles (docs/02, «Событие и лог»): the application role may only
-- read and append to the event log; only the erasure role may run
-- erase_learner(). Roles belong to the cluster, not to this database, so
-- they are created if missing and never dropped by a down migration.
DO $$
BEGIN
    CREATE ROLE refl_app NOLOGIN;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$
BEGIN
    CREATE ROLE refl_erasure NOLOGIN;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

GRANT USAGE ON SCHEMA public TO refl_app, refl_erasure;
