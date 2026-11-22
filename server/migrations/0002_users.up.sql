-- Users, roles, mentor links, login methods and privacy settings
-- (docs/01, «Пользователи и соцграф в схеме»).

-- Random 8-character Crockford base32 code a person can type.
CREATE DOMAIN public_code AS text CHECK (VALUE ~ '^[0-9A-HJKMNP-TV-Z]{8}$');

CREATE TABLE users (
    id uuid PRIMARY KEY,
    public_code public_code NOT NULL UNIQUE,
    display_name text NOT NULL,
    interface_lang text NOT NULL CHECK (interface_lang IN ('uz', 'ru', 'en')),
    -- Set by the adult who creates a child's account; the child cannot clear it.
    minor boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE user_roles (
    user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role text NOT NULL CHECK (role IN ('learner', 'mentor', 'author', 'admin')),
    PRIMARY KEY (user_id, role)
);

-- An adult learner accepts a mentor (pending -> active); a child's mentor
-- link is the guardian link, created active with the account.
CREATE TABLE mentor_links (
    mentor_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    learner_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    guardian boolean NOT NULL DEFAULT false,
    status text NOT NULL CHECK (status IN ('pending', 'active')),
    created_at timestamptz NOT NULL DEFAULT now(),
    accepted_at timestamptz,
    PRIMARY KEY (mentor_id, learner_id),
    CHECK (mentor_id <> learner_id),
    CHECK ((status = 'active') = (accepted_at IS NOT NULL)),
    CHECK (NOT guardian OR status = 'active')
);
CREATE INDEX mentor_links_learner ON mentor_links (learner_id);

-- One account, several login methods. Email: verified address, no password.
-- Telegram: the Telegram user id.
CREATE TABLE login_methods (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    kind text NOT NULL CHECK (kind IN ('email', 'telegram')),
    identifier text NOT NULL,
    verified_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (kind, identifier),
    CHECK (kind <> 'email' OR identifier = lower(identifier)),
    CHECK (kind <> 'telegram' OR identifier ~ '^[0-9]+$')
);
CREATE INDEX login_methods_user ON login_methods (user_id);

-- Short-lived one-time email codes or links, stored only as a hash.
CREATE TABLE login_tokens (
    token_hash bytea PRIMARY KEY,
    email text NOT NULL CHECK (email = lower(email)),
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL,
    consumed_at timestamptz,
    CHECK (expires_at > created_at)
);

CREATE TABLE privacy_settings (
    user_id uuid PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    profile_visibility text NOT NULL
        CHECK (profile_visibility IN ('private', 'team', 'friends', 'public')),
    friend_requests_from text NOT NULL
        CHECK (friend_requests_from IN ('nobody', 'team', 'anyone')),
    updated_at timestamptz NOT NULL DEFAULT now()
);

-- Every user gets privacy settings with defaults that are stricter for minors.
CREATE FUNCTION users_privacy_defaults() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO privacy_settings (user_id, profile_visibility, friend_requests_from)
    VALUES (
        NEW.id,
        CASE WHEN NEW.minor THEN 'private' ELSE 'friends' END,
        CASE WHEN NEW.minor THEN 'team' ELSE 'anyone' END
    );
    RETURN NEW;
END $$;

CREATE TRIGGER users_privacy_defaults AFTER INSERT ON users
    FOR EACH ROW EXECUTE FUNCTION users_privacy_defaults();

GRANT SELECT, INSERT, UPDATE, DELETE
    ON users, user_roles, mentor_links, login_methods, login_tokens, privacy_settings
    TO refl_app;
