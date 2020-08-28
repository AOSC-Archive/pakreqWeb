-- This file should undo anything in `up.sql`

DROP TRIGGER IF EXISTS date_limit_check ON public."iter_entries" CASCADE;
DROP TABLE public."iter_entries" CASCADE;
DROP TABLE public."iter_plans" CASCADE;
