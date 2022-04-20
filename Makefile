SERVER_TY=email

test:
	cargo test --all-features -- --test-threads=1 

docs:
	BUILD_ENABLED=0 cargo doc --no-deps
	rm -rf ./docs
	echo "<meta http-equiv=\"refresh\" content=\"0; url=simple_syrup\">" > target/doc/index.html
	cp -r target/doc ./docs

prepare:
	mkcert localhost
	cp .env.local .env
	psql postgres -f ./init-dbs.sql

deploy:
	node deploy.js

local:
	SERVER_TY=\"QA\" ACTIVE_SERVERS=["{\"server_ty\": \"QA\", \"url\": \"http://127.0.0.1:8080\"}"] cargo run --features qa
