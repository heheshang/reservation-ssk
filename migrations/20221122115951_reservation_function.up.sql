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
    -- if page_size is not between 1 and 100, set it to 10
    IF page_size < 1 OR page_size > 100 THEN
        page_size := 10;
    END IF;
    -- if page is not positive, set it to 1
    IF page < 1 THEN
        page := 1;
    END IF;
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


-- we filter 2 more items one for starting ,one for ending .
-- If starting existing,then we have previous page,if ending existing,then we have next page
CREATE OR REPLACE FUNCTION rsvp.filter(
    uid text,
    rid text,
    status rsvp.reservation_status DEFAULT 'pending',
    cursor bigint DEFAULT NULL,
    is_desc bool DEFAULT false,
    page_size integer DEFAULT 10
    ) RETURNS TABLE(like rsvp.reservations) AS $$
DECLARE
    _sql text;
    _offset bigint;
BEGIN

    -- if page_size is not between 1 and 100, set it to 10
    IF page_size < 1 OR page_size > 100 THEN
        page_size := 10;
    END IF;

    -- if cursor is null or less than 0, set it to 0 if is_desc is false, or to max(bigint) if is_desc is true
    IF cursor IS NULL OR cursor < 0 THEN
        IF is_desc THEN
            cursor := max(bigint);
        ELSE
            cursor := 0;
        END IF;
    END IF;


    -- format the query based on parameters
    _sql := format(
        'SELECT * FROM rsvp.reservations where %s AND status = %L AND %s ORDER BY id %s  LIMIT %L::integer',
        CASE
            WHEN is_desc THEN 'id <= ' || cursor
            ELSE 'id >= ' || cursor
        END,
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
        page_size+1
    );
    -- log the sql
    RAISE NOTICE '%', _sql;
    -- execute the query
    RETURN QUERY EXECUTE _sql ;
END;

$$ LANGUAGE plpgsql;
