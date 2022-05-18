-- Add migration script here

ALTER TABLE prepare ADD COLUMN contract_address VARCHAR;

ALTER TABLE authenticated ADD COLUMN contract_address VARCHAR;