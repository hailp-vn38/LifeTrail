-- A timezone is a projection target, not Raw input.  Changing it therefore
-- schedules each Device for projection-only work without touching
-- input_generation or dirty activity range state.
CREATE FUNCTION queue_timezone_reprojection() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.timezone IS DISTINCT FROM OLD.timezone THEN
        INSERT INTO processing_days(device_id, local_date, timezone, state)
        SELECT d.id, (g.recorded_at AT TIME ZONE NEW.timezone)::date, NEW.timezone, 'queued'
        FROM devices d JOIN gps_points g ON g.device_id=d.id
        WHERE d.owner_user_id=NEW.id
        GROUP BY d.id, (g.recorded_at AT TIME ZONE NEW.timezone)::date
        ON CONFLICT(device_id,local_date,timezone) DO UPDATE SET state='queued';

        UPDATE device_processing_control c
        SET work_generation=work_generation+1
        FROM devices d
        WHERE d.id=c.device_id AND d.owner_user_id=NEW.id;

        INSERT INTO processing_jobs(device_id,state)
        SELECT c.device_id, 'queued'
        FROM device_processing_control c JOIN devices d ON d.id=c.device_id
        WHERE d.owner_user_id=NEW.id
        ON CONFLICT(device_id) DO UPDATE SET
            state=CASE WHEN processing_jobs.state='running' THEN 'running' ELSE 'queued' END,
            failure_message=NULL;
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER owner_timezone_reprojection AFTER UPDATE OF timezone ON users
FOR EACH ROW EXECUTE FUNCTION queue_timezone_reprojection();
