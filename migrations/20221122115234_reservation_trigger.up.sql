CREATE TABLE rsvp.reservation_changes (
    id SERIAL not null,
    reservation_id bigserial NOT NULL,
    op rsvp.reservation_update_type NOT NULL ,
    CONSTRAINT reservation_changes_pkey PRIMARY KEY (id)
);

-- trigger for add/update/delete a reservation
CREATE OR REPLACE FUNCTION rsvp.reservation_trigger() RETURNS trigger AS $$
BEGIN
    IF (TG_OP = 'INSERT') THEN
        -- update reservation_changes table
        INSERT INTO rsvp.reservation_changes (reservation_id, op) VALUES (NEW.id, 'create');
        -- check if the reservation is valid
        -- check if the reservation is conflicting with other reservations
        -- if not, insert the reservation
        -- if yes, return error
    ELSIF (TG_OP = 'UPDATE') THEN
        -- if status is changed, update reservation_changes table
        IF (OLD.status != NEW.status) THEN
            INSERT INTO rsvp.reservation_changes (reservation_id, op) VALUES (NEW.id, 'update');
        END IF;

        -- check if the reservation is valid
        -- check if the reservation is conflicting with other reservations
        -- if not, update the reservation
        -- if yes, return error
    ELSIF (TG_OP = 'DELETE') THEN
        -- update reservation_changes table
        INSERT INTO rsvp.reservation_changes (reservation_id, op) VALUES (OLD.id, 'delete');

        -- delete the reservation
    END IF;
    -- notify a channel called reservation_change
    NOTIFY reservation_update;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER reservation_trigger
    AFTER INSERT OR UPDATE OR DELETE ON rsvp.reservations
    FOR EACH ROW EXECUTE PROCEDURE rsvp.reservation_trigger();
