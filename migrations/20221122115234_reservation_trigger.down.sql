DROP TRIGGER IF EXISTS reservation_trigger ON rsvp.reservations;
DROP FUNCTION IF EXISTS rsvp.reservation_trigger;
DROP TABLE rsvp.reservation_changes CASCADE;
DROP TABLE rsvp.server_read_cursor CASCADE;
