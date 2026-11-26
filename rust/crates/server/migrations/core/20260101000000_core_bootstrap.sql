CREATE SCHEMA IF NOT EXISTS "public";
CREATE SCHEMA IF NOT EXISTS "core";

COMMENT ON SCHEMA "public" IS 'standard public schema';

CREATE EXTENSION IF NOT EXISTS "uuid-ossp" SCHEMA public;
CREATE EXTENSION IF NOT EXISTS "unaccent" SCHEMA public;
CREATE EXTENSION IF NOT EXISTS "citext" SCHEMA public;

ALTER EXTENSION "uuid-ossp" SET SCHEMA public;
ALTER EXTENSION "unaccent" SET SCHEMA public;
ALTER EXTENSION "citext" SET SCHEMA public;

CREATE OR REPLACE FUNCTION public.unaccent_immutable(input text)
    RETURNS text
    LANGUAGE sql
    IMMUTABLE
AS $$
        SELECT public.unaccent('public.unaccent'::regdictionary, input)
        $$;
