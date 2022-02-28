prepare:
	mkcert localhost
	cp .env.local .env
	cp .env.local .env.docker