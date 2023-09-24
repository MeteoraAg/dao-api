-- Add down migration script here
-- DROP MATERIALIZED VIEW IF EXISTS epoch_gauge;

DROP TABLE IF EXISTS public.token;
DROP TABLE IF EXISTS public.gauge;
DROP TABLE IF EXISTS public.epoch_gauge;