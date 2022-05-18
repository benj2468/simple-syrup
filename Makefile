SERVER_TY=email

test:
	cargo test --features test -- --test-threads=1 

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
	WEB3_HOST="ws://127.0.0.1:7545" ETH_ADDRESS=0xE246D524588888898A0567fAa898A775E86cBD89 SERVER_TY=\"Email\" ACTIVE_SERVERS="[{\"server_ty\": \"QA\", \"url\": \"http://127.0.0.1:8080\"}, {\"server_ty\": \"Email\", \"url\": \"http://127.0.0.1:8081\"}]" cargo run --features email --features web3 --features development
