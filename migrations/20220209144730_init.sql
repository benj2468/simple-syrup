-- Add migration script here
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE TYPE VerificationStatus AS ENUM ('Requested', 'Verified', 'RequestAuth');
CREATE TABLE IF NOT EXISTS authenticated (
  id UUID DEFAULT uuid_generate_v4 (),
  data jsonb UNIQUE PRIMARY KEY NOT NULL,
  status VerificationStatus NOT NULL,
  secret_component VARCHAR
);