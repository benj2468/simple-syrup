-- Add migration script here
ALTER TABLE
  prepare
ADD
  COLUMN data jsonb;