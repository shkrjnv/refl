DROP TABLE auth_sessions;
DROP INDEX login_tokens_email_created;
ALTER TABLE login_tokens DROP COLUMN attempts;
