prepare:
	mkcert localhost
	cp .env.local .env
	psql postgres -f ./init-dbs.sql