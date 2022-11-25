-- if user_id is not provided, we can use this index to query all reservations for a given resource in a given during time range
-- if resource_id is not provided, we can use this index to query all reservations for a given user in a given during time range
-- if both resource_id and user_id are provided, we can use this index to query all reservations for a given resource and user in a given during time range
-- if neither resource_id nor user_id are provided, we can use this index to query all reservations in a given during time range
--
--explain analyze select * from  rsvp.query('ssk',null,'("2021-11-01","2022-12-31")');
-- INSERT into rsvp.reservations(user_id, resource_id, timespan) VALUES( 'ssk',  'room-404', '("2021-12-02","2021-12-31")');

CREATE OR REPLACE FUNCTION rsvp.query(
    uid text,
    rid text,
    during tstzrange ,
    status rsvp.reservation_status DEFAULT 'pending',
    page integer DEFAULT 1,
    is_desc bool DEFAULT false,
    page_size integer DEFAULT 10
    ) RETURNS TABLE(like rsvp.reservations) AS $$
DECLARE
    _sql text;
BEGIN
    -- format the query based on parameters
    _sql := format(
        'SELECT * FROM rsvp.reservations WHERE %L @> timespan  AND status = %L AND %s ORDER BY lower(timespan)
        %s  LIMIT %s OFFSET  %L::integer',
        during,
        status,
        CASE
            WHEN uid IS NULL AND rid IS NULL THEN 'TRUE'
            WHEN uid IS NULL THEN 'resource_id = ' || quote_literal(rid)
            WHEN rid IS NULL THEN 'user_id = ' || quote_literal(uid)
            ELSE 'resource_id = ' || quote_literal(rid) || ' AND user_id = ' || quote_literal(uid)
        END,
        CASE
            WHEN is_desc THEN 'DESC'
            ELSE 'ASC'
        END,
        page_size,
        (page - 1) * page_size
    );
    -- log the sql
    RAISE NOTICE '%', _sql;
    -- execute the query
    RETURN QUERY EXECUTE _sql ;
END;

$$ LANGUAGE plpgsql;
