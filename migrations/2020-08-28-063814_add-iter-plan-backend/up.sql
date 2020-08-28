-- 
CREATE TABLE public."iter_plans"(
    id bigserial PRIMARY KEY NOT NULL,
    title text NOT NULL,
    begin_date timestamp NOT NULL,
    end_date timestamp NOT NULL,
    notes text NOT NULL
);

CREATE TABLE public."iter_entries"(
    id bigserial PRIMARY KEY NOT NULL,
    plan_id bigserial NOT NULL REFERENCES "iter_plans"(id),
    title text NOT NULL,
    done bool NOT NULL,
    "date" timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    category text NOT NULL,
    "version" text NOT NULL,
    origin text NOT NULL,
    "target" text NOT NULL,
    "description" text NOT NULL
);

CREATE OR REPLACE FUNCTION check_entry_date() 
RETURNS trigger AS $$
    BEGIN
        IF NOT (SELECT NEW.date BETWEEN begin_date AND end_date FROM iter_plans WHERE id = NEW.plan_id) THEN
            RAISE EXCEPTION 'Entry date is not between the begin and end of its plan';
        END IF;
        RETURN NEW;
    END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER date_limit_check
BEFORE UPDATE OF date OR INSERT ON iter_entries
FOR EACH ROW EXECUTE PROCEDURE check_entry_date();
