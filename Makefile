SERVER_TY=email

test:
	cargo test -- --test-threads=1

prepare:
	mkcert localhost
	cp .env.local .env
	psql postgres -f ./init-dbs.sql

deploy:
	node deploy.js

local:
	SERVER_TY=\"QA\" ACTIVE_SERVERS=["{\"server_ty\": \"QA\", \"url\": \"http://localhost:8080\"}"] cargo run --features qa