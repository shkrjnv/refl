-- Access/refresh sessions (docs/13, «Вход и токены»): only SHA-256 digests
-- are stored, never the bearer tokens themselves.
CREATE TABLE auth_sessions (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    access_token_hash bytea NOT NULL UNIQUE,
    refresh_token_hash bytea NOT NULL UNIQUE,
    access_expires_at timestamptz NOT NULL,
    refresh_expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    revoked_at timestamptz,
    CHECK (access_expires_at > created_at),
    CHECK (refresh_expires_at > access_expires_at)
);
CREATE INDEX auth_sessions_user ON auth_sessions (user_id);

-- One code per email is stored; attempts bound brute force. Codes themselves
-- are stored only as SHA-256 digests in login_tokens.token_hash.
ALTER TABLE login_tokens ADD COLUMN attempts integer NOT NULL DEFAULT 0
    CHECK (attempts >= 0);
CREATE INDEX login_tokens_email_created ON login_tokens (email, created_at DESC);

GRANT SELECT, INSERT, UPDATE, DELETE ON auth_sessions TO refl_app;
