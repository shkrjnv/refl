DROP FUNCTION erase_account(uuid);
DROP FUNCTION direct_message_allowed(uuid, uuid);
DROP TRIGGER privacy_minor_guard ON privacy_settings;
DROP FUNCTION minor_privacy_guard();
