DROP TABLE invites;
DROP TABLE roles;
DROP INDEX users_access_status;
ALTER TABLE users DROP COLUMN access_status;
