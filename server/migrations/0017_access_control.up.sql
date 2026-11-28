-- Access control (docs/01, «Пользователи и соцграф в схеме»): every user has
-- an access status; invites are stored only by SHA-256 digest.
ALTER TABLE users ADD COLUMN access_status text NOT NULL DEFAULT 'active'
    CHECK (access_status IN ('active', 'pending', 'rejected'));
CREATE INDEX users_access_status ON users (access_status);

-- The catalogue of roles; admin access is checked through user_roles.
CREATE TABLE roles (
    role text PRIMARY KEY
        CHECK (role IN ('learner', 'mentor', 'author', 'admin'))
);
INSERT INTO roles (role) VALUES ('learner'), ('mentor'), ('author'), ('admin')
    ON CONFLICT DO NOTHING;
GRANT SELECT ON roles TO refl_app;

CREATE TABLE invites (
    id uuid PRIMARY KEY,
    code_hash text NOT NULL UNIQUE
        CHECK (code_hash ~ '^[0-9a-f]{64}$'),
    created_by uuid REFERENCES users (id),
    note text,
    max_uses integer NOT NULL DEFAULT 1 CHECK (max_uses > 0),
    used_count integer NOT NULL DEFAULT 0 CHECK (used_count >= 0),
    expires_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX invites_created_by ON invites (created_by);

GRANT SELECT, INSERT, UPDATE ON invites TO refl_app;
