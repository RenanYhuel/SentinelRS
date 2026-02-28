.PHONY: build-all fmt proto compose-up compose-down

build-all:
	cargo build --workspace

fmt:
	cargo fmt --all

proto:
	cargo build -p sentinel_common

compose-up:
	docker-compose -f deploy/docker-compose.yml up -d

compose-down:
	docker-compose -f deploy/docker-compose.yml down
