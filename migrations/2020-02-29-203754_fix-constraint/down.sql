-- This file should undo anything in `up.sql`
BEGIN TRANSACTION;
ALTER TABLE public.oauth DROP CONSTRAINT IF EXISTS oauthtype;
ALTER TABLE public.request DROP CONSTRAINT IF EXISTS request_packager_id_fkey;
UPDATE request SET packager_id = 0 WHERE packager_id = NULL;
ALTER TABLE public."user" ALTER COLUMN username DROP NOT NULL;
ALTER TABLE public."user" ALTER COLUMN "admin" DROP NOT NULL;
ALTER TABLE public.request ALTER COLUMN "status" DROP NOT NULL;
ALTER TABLE public.request ALTER COLUMN "type" DROP NOT NULL;
ALTER TABLE public.request ALTER COLUMN "name" DROP NOT NULL;
ALTER TABLE public.request ALTER COLUMN "requester_id" DROP NOT NULL;
ALTER TABLE public.request ALTER COLUMN "pub_date" DROP NOT NULL;
END TRANSACTION;
