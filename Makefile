.PHONY: build-all fmt fmt-check proto compose-up compose-down \
       test test-unit test-integration lint bench \
       e2e docker-build release-cli clean doc

build-all:
	cargo build --workspace

test: test-unit test-integration

test-unit:
	cargo test --workspace --lib --bins

test-integration:
	cargo test --workspace --test '*'

lint:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

bench:
	cargo bench --workspace

doc:
	cargo doc --workspace --no-deps

proto:
	cargo build -p sentinel_common

compose-up:
	docker-compose -f deploy/docker-compose.yml up -d

compose-down:
	docker-compose -f deploy/docker-compose.yml down

e2e:
	bash tests/e2e/run_e2e.sh

docker-build:
	docker build -f deploy/Dockerfile -t sentinelrs:latest .

release-cli:
	cargo build --release -p sentinel_cli

release-all:
	cargo build --release --workspace

clean:
	cargo clean
