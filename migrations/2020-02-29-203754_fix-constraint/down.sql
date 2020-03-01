-- This file should undo anything in `up.sql`
BEGIN TRANSACTION;
ALTER TABLE public.oauth DROP CONSTRAINT IF EXISTS oauthtype;
ALTER TABLE public."user" ALTER COLUMN username SET NULL;
ALTER TABLE public."user" ALTER COLUMN "admin" SET NULL;
ALTER TABLE public.request ALTER COLUMN "status" SET NULL;
ALTER TABLE public.request ALTER COLUMN "type" SET NULL;
ALTER TABLE public.request ALTER COLUMN "name" SET NULL;
ALTER TABLE public.request ALTER COLUMN "requester_id" SET NULL;
ALTER TABLE public.request ALTER COLUMN "pub_date" SET NULL;
END TRANSACTION;
