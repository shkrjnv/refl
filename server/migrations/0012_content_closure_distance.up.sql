-- Shortest distance in `requires` edges from kc to required_kc, for the
-- propagation of success (docs/03, «Распространение по графу»). NULL only for
-- versions imported from bundles before format 5, which had no distances.
ALTER TABLE content_requires_closure
    ADD COLUMN distance integer CHECK (distance >= 1);
