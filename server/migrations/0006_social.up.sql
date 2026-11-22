-- Social graph (docs/01, «Социальные функции и инструменты», «Пользователи и
-- соцграф в схеме»): teams, membership, invites, friendships, blocks, reports.
CREATE TABLE teams (
    id uuid PRIMARY KEY,
    public_code public_code NOT NULL UNIQUE,
    name text NOT NULL,
    -- Any user, including a mentor.
    owner_id uuid NOT NULL REFERENCES users (id),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE team_members (
    team_id uuid NOT NULL REFERENCES teams (id) ON DELETE CASCADE,
    user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role text NOT NULL CHECK (role IN ('owner', 'member')),
    joined_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (team_id, user_id)
);
CREATE UNIQUE INDEX team_members_one_owner ON team_members (team_id) WHERE role = 'owner';
CREATE INDEX team_members_user ON team_members (user_id);

-- Revocable invite links, stored only as a hash.
CREATE TABLE team_invites (
    token_hash bytea PRIMARY KEY,
    team_id uuid NOT NULL REFERENCES teams (id) ON DELETE CASCADE,
    created_by uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz,
    revoked_at timestamptz
);

-- A friendship is an accepted request; one request per pair, either direction.
CREATE TABLE friend_requests (
    requester_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    addressee_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    status text NOT NULL CHECK (status IN ('pending', 'accepted', 'declined')),
    created_at timestamptz NOT NULL DEFAULT now(),
    responded_at timestamptz,
    PRIMARY KEY (requester_id, addressee_id),
    CHECK (requester_id <> addressee_id),
    CHECK ((status = 'pending') = (responded_at IS NULL))
);
CREATE UNIQUE INDEX friend_requests_pair
    ON friend_requests (least(requester_id, addressee_id), greatest(requester_id, addressee_id));
CREATE INDEX friend_requests_addressee ON friend_requests (addressee_id);

CREATE TABLE blocks (
    blocker_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    blocked_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (blocker_id, blocked_id),
    CHECK (blocker_id <> blocked_id)
);

-- A report targets a user; the optional context (kind, id) will point at a
-- chat message or a team once those exist.
CREATE TABLE reports (
    id uuid PRIMARY KEY,
    reporter_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    reported_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    reason text NOT NULL,
    context_kind text,
    context_id text,
    status text NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'resolved')),
    resolved_by uuid REFERENCES users (id) ON DELETE SET NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    resolved_at timestamptz,
    CHECK (reporter_id <> reported_id),
    CHECK ((context_kind IS NULL) = (context_id IS NULL)),
    CHECK ((status = 'resolved') = (resolved_at IS NOT NULL))
);
CREATE INDEX reports_open ON reports (created_at) WHERE status = 'open';

GRANT SELECT, INSERT, UPDATE, DELETE
    ON teams, team_members, team_invites, friend_requests, blocks, reports
    TO refl_app;
