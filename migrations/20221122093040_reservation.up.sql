create type rsvp.reservation_status as enum ('unknown', 'pending', 'confirmed', 'blocked');
create type rsvp.reservation_update_type as enum ('unknown', 'create', 'update', 'delete');

CREATE TABLE rsvp.reservations (
    id bigserial NOT NULL ,
    user_id varchar(64) NOT NULL,
    status rsvp.reservation_status NOT NULL  DEFAULT 'pending',
    resource_id varchar(64) NOT NULL,
    timespan tstzrange NOT NULL,
    note text,
    CONSTRAINT reservations_pkey PRIMARY KEY (id),
    CONSTRAINT reservations_conflict EXCLUDE USING gist (resource_id WITH =, timespan WITH &&)
);

CREATE INDEX reservation_resource_id_idx ON rsvp.reservations (resource_id);
CREATE INDEX reservation_user_id_idx ON rsvp.reservations (user_id);
