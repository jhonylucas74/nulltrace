.PHONY: dev test client core stress help

help:
	@echo "Nulltrace - Makefile"
	@echo ""
	@echo "Available commands:"
	@echo "  make dev     - Run frontend and backend in parallel"
	@echo "  make test    - Run core tests (requires PostgreSQL)"
	@echo "  make client  - Run frontend only"
	@echo "  make core    - Run backend only"
	@echo "  make stress  - Run stress test with 5k VMs (release mode)"

dev:
	@echo "Starting frontend and backend..."
	@$(MAKE) -j2 client core

client:
	@echo "Starting frontend (nulltrace-client)..."
	@cd nulltrace-client && npm run tauri dev

core:
	@echo "Starting backend (nulltrace-core) in GAME MODE..."
	@cd nulltrace-core && cargo run --release --bin cluster

test:
	@echo "Running tests (PostgreSQL required at postgres://nulltrace:nulltrace@localhost:5432/nulltrace)..."
	@cd nulltrace-core && cargo test --bin cluster -- --test-threads=1

stress:
	@echo "Running stress test (5000 VMs, 20s, release mode)..."
	@cd nulltrace-core && STRESS_TEST=1 cargo run --release --bin cluster
