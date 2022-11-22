create type rsvp.reservation_status as enum ('unknown', 'pending', 'confirmed', 'blocked');
create type rsvp.reservation_update_type as enum ('unknown', 'create', 'update', 'delete');

CREATE TABLE rsvp.reservations (
    id uuid NOT NULL DEFAULT gen_random_uuid(),
    user_id varchar(64) NOT NULL,
    status rsvp.reservation_status NOT NULL  DEFAULT 'pending',
    resource_id varchar(64) NOT NULL,
    timespan tstzrange NOT NULL,
    note text,
    CONSTRAINT reservations_pkey PRIMARY KEY (id)
);

CREATE INDEX reservation_resource_id_idx ON rsvp.reservations (resource_id);
CREATE INDEX reservation_user_id_idx ON rsvp.reservations (user_id);
