--
-- PostgreSQL database dump
--

\restrict LwL8XTkREjYmTSZCQHaVy8EdwiE1HOKveF4Whx4L1xF2Ad7xQMx0J3ElE52qT54

-- Dumped from database version 16.16 (Debian 16.16-1.pgdg13+1)
-- Dumped by pg_dump version 16.16 (Debian 16.16-1.pgdg13+1)

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: public; Type: SCHEMA; Schema: -; Owner: -
--

CREATE SCHEMA public;


--
-- Name: SCHEMA public; Type: COMMENT; Schema: -; Owner: -
--

COMMENT ON SCHEMA public IS 'standard public schema';


--
-- Name: unaccent_immutable(text); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.unaccent_immutable(input text) RETURNS text
    LANGUAGE sql IMMUTABLE
    AS $$
        SELECT public.unaccent('public.unaccent'::regdictionary, input)
        $$;


--
-- PostgreSQL database dump complete
--

\unrestrict LwL8XTkREjYmTSZCQHaVy8EdwiE1HOKveF4Whx4L1xF2Ad7xQMx0J3ElE52qT54

