-- Child privacy/chat rules (docs/01, docs/14), and full account erasure for
-- the authenticated self-service endpoint (docs/02, docs/14).
--
-- A minor's profile never becomes public and a minor never accepts friend
-- requests from anyone; direct messages are allowed only when neither side is
-- a minor. Team chat is governed by team membership, not this function.
-- Migrate existing minor profiles to the strict defaults before the guard
-- prevents unsafe updates.
UPDATE privacy_settings
SET profile_visibility = 'private', friend_requests_from = 'team',
    updated_at = now()
WHERE user_id IN (SELECT id FROM users WHERE minor);

CREATE FUNCTION minor_privacy_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    is_minor boolean;
BEGIN
    SELECT minor INTO is_minor FROM users WHERE id = NEW.user_id;
    IF is_minor AND (
        NEW.profile_visibility NOT IN ('private', 'team')
        OR NEW.friend_requests_from = 'anyone'
    ) THEN
        RAISE EXCEPTION 'a minor profile must stay private or team-only'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END $$;

CREATE TRIGGER privacy_minor_guard
    BEFORE INSERT OR UPDATE ON privacy_settings
    FOR EACH ROW EXECUTE FUNCTION minor_privacy_guard();

CREATE FUNCTION direct_message_allowed(sender uuid, recipient uuid)
    RETURNS boolean LANGUAGE sql STABLE AS $$
    SELECT sender <> recipient AND NOT EXISTS (
        SELECT 1 FROM users WHERE id IN (sender, recipient) AND minor
    )
$$;

REVOKE ALL ON FUNCTION direct_message_allowed(uuid, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION direct_message_allowed(uuid, uuid) TO refl_app;

-- Erase the learning log first (the only permitted delete path), then the
-- account's personal rows. The erasures row keeps only the learner id and
-- time; no email, name, Telegram id, token or event survives.
CREATE FUNCTION erase_account(learner uuid) RETURNS timestamptz
    LANGUAGE plpgsql SECURITY DEFINER SET search_path FROM CURRENT AS $$
DECLARE
    erased_at timestamptz;
BEGIN
    PERFORM erase_learner(learner);
    DELETE FROM login_tokens WHERE email IN (
        SELECT identifier FROM login_methods
        WHERE user_id = learner AND kind = 'email'
    );
    UPDATE invites SET created_by = NULL WHERE created_by = learner;
    DELETE FROM teams WHERE owner_id = learner;
    DELETE FROM users WHERE id = learner;
    SELECT e.erased_at INTO erased_at FROM erasures e WHERE e.learner_id = learner;
    RETURN erased_at;
END $$;

REVOKE ALL ON FUNCTION erase_account(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION erase_account(uuid) TO refl_app;
