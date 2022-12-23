CREATE TABLE rsvp.reservation_changes (
    id SERIAL not null,
    reservation_id bigserial NOT NULL,
    old JSONB,
    new JSONB,
    op rsvp.reservation_update_type NOT NULL ,
    CONSTRAINT reservation_changes_pkey PRIMARY KEY (id)
);


-- create index on reservation_changes with reservation_id and op
CREATE INDEX reservation_changes_reservation_id_op_idx ON rsvp.reservation_changes (reservation_id,op);

-- server read cursor

CREATE TABLE rsvp.server_read_cursor(
    server_id VARCHAR(64) NOT NULL,
    last_change_id BIGSERIAL NOT NULL,
    CONSTRAINT reservation_changes_cursor_pkey PRIMARY KEY (server_id)
);



-- trigger for add/update/delete a reservation
CREATE OR REPLACE FUNCTION rsvp.reservation_trigger() RETURNS trigger AS $$
BEGIN
    IF (TG_OP = 'INSERT') THEN
        -- update reservation_changes table
        INSERT INTO rsvp.reservation_changes (reservation_id,old,new,op) VALUES (NEW.id,NULL,to_jsonb(NEW), 'create');
        -- check if the reservation is valid
        -- check if the reservation is conflicting with other reservations
        -- if not, insert the reservation
        -- if yes, return error
    ELSIF (TG_OP = 'UPDATE') THEN
        -- if status is changed, update reservation_changes table
        IF (OLD.status != NEW.status) THEN
            INSERT INTO rsvp.reservation_changes (reservation_id,old,new,op) VALUES (NEW.id,to_jsonb(OLD),to_jsonb(NEW), 'update');
        END IF;

        -- check if the reservation is valid
        -- check if the reservation is conflicting with other reservations
        -- if not, update the reservation
        -- if yes, return error
    ELSIF (TG_OP = 'DELETE') THEN
        -- update reservation_changes table
        INSERT INTO rsvp.reservation_changes (reservation_id,old,new,op) VALUES (OLD.id,to_jsonb(OLD),NULL, 'delete');

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
