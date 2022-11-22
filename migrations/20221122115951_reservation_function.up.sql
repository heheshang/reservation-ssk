-- if user_id is not provided, we can use this index to query all reservations for a given resource in a given during time range
-- if resource_id is not provided, we can use this index to query all reservations for a given user in a given during time range
-- if both resource_id and user_id are provided, we can use this index to query all reservations for a given resource and user in a given during time range
-- if neither resource_id nor user_id are provided, we can use this index to query all reservations in a given during time range
CREATE OR REPLACE FUNCTION rsvp.query(uid text, rid text,during tstzrange ) RETURNS TABLE
(like rsvp.reservations) AS $$
BEGIN
    IF (uid IS NULL AND rid IS NULL) THEN
        RETURN QUERY SELECT * FROM rsvp.reservations WHERE during && timespan;
    ELSIF (uid IS NULL) THEN
        RETURN QUERY SELECT * FROM rsvp.reservations WHERE during @> timespan AND resource_id = rid;
    ELSIF (rid IS NULL) THEN
        RETURN QUERY SELECT * FROM rsvp.reservations WHERE during @> timespan AND user_id = uid;
    ELSE
        RETURN QUERY SELECT * FROM rsvp.reservations WHERE during @> timespan AND resource_id = rid AND user_id = uid;
    END IF;
END;

$$ LANGUAGE plpgsql;
