-- Notifications module (docs/01, «Пользователи и соцграф в схеме»): channels,
-- per (kind, channel) preferences and an outbox. Later stages add kinds by
-- migration.
CREATE DOMAIN notification_channel AS text CHECK (VALUE IN ('telegram', 'push', 'email'));
CREATE DOMAIN notification_kind AS text
    CHECK (VALUE IN ('review_reminder', 'daily_summary', 'mentor_update'));

-- telegram: {"chat_id": ...}; push: a web push subscription (one per device);
-- email: {"address": ...}.
CREATE TABLE notification_channels (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    channel notification_channel NOT NULL,
    target jsonb NOT NULL,
    verified_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX notification_channels_single
    ON notification_channels (user_id, channel) WHERE channel <> 'push';

CREATE TABLE notification_preferences (
    user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    kind notification_kind NOT NULL,
    channel notification_channel NOT NULL,
    enabled boolean NOT NULL,
    PRIMARY KEY (user_id, kind, channel)
);

CREATE TABLE notification_outbox (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    channel notification_channel NOT NULL,
    kind notification_kind NOT NULL,
    payload jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    next_attempt_at timestamptz NOT NULL DEFAULT now(),
    sent_at timestamptz,
    attempts integer NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    last_error text
);
CREATE INDEX notification_outbox_due ON notification_outbox (next_attempt_at) WHERE sent_at IS NULL;

GRANT SELECT, INSERT, UPDATE, DELETE
    ON notification_channels, notification_preferences, notification_outbox
    TO refl_app;
