.PHONY: dev test client core help

help:
	@echo "Nulltrace - Makefile"
	@echo ""
	@echo "Available commands:"
	@echo "  make dev    - Run frontend and backend in parallel"
	@echo "  make test   - Run core tests (requires PostgreSQL)"
	@echo "  make client - Run frontend only"
	@echo "  make core   - Run backend only"

dev:
	@echo "Starting frontend and backend..."
	@$(MAKE) -j2 client core

client:
	@echo "Starting frontend (nulltrace-client)..."
	@cd nulltrace-client && npm run tauri dev

core:
	@echo "Starting backend (nulltrace-core)..."
	@cd nulltrace-core && cargo run --bin cluster

test:
	@echo "Running tests (PostgreSQL required at postgres://nulltrace:nulltrace@localhost:5432/nulltrace)..."
	@cd nulltrace-core && cargo test --bin cluster -- --test-threads=1
