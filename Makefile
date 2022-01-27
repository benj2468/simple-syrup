build:
	solc --optimize-runs 1 --bin --abi -o build ./contracts/* --overwrite

deploy:
	cd deployer && cargo run

