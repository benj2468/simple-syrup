-- Add migration script here
CREATE TABLE IF NOT EXISTS prepare (
  id UUID DEFAULT uuid_generate_v4 (),
  email VARCHAR NOT NULL,
  secret_component VARCHAR
)