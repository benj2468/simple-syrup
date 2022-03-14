prepare:
	mkcert localhost
	cp .env.local .env
	psql postgres -f ./init-dbs.sql


run:
	DATABASE_URL=postgres://localhost:5432/cpass cargo run
build:
	DATABASE_URL=postgres://localhost:5432/cpass cargo build

deploy:
	SERVER_COUNT=2 node predeploy.js
	git push heroku multi:main