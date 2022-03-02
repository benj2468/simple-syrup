prepare:
	mkcert localhost
	cp .env.local .env
	psql postgres -f ./init-dbs.sql


run:
	DATABASE_URL=postgres://localhost:5432/cpass cargo run