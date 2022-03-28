-- Add migration script here
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE TYPE VerificationStatus AS ENUM ('Requested', 'Verified', 'RequestAuth');
CREATE TABLE IF NOT EXISTS authenticated (
  id UUID DEFAULT uuid_generate_v4 (),
  email VARCHAR UNIQUE PRIMARY KEY NOT NULL,
  data jsonb,
  status VerificationStatus NOT NULL,
  secret_component VARCHAR
);