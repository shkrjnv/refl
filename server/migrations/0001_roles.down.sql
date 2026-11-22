-- Roles are cluster objects shared by every database of the cluster: they
-- are not dropped here (docs/02, «Событие и лог»).
REVOKE USAGE ON SCHEMA public FROM refl_app, refl_erasure;
