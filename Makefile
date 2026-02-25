.PHONY: dev debug test test-ntml client client-debug core core-debug stress reset help

help:
	@echo "Nulltrace - Makefile"
	@echo ""
	@echo "Available commands:"
	@echo "  make reset     - Reset Docker (down -v, up -d) and run make dev"
	@echo "  make dev       - Run frontend and backend in parallel"
	@echo "  make debug     - Like make dev but with Tauri DevTools + cluster tick logs"
	@echo "  make test      - Run all tests (core + ntml)"
	@echo "  make test-ntml - Run NTML parser tests only"
	@echo "  make client    - Run frontend only"
	@echo "  make core      - Run backend only (no tick spam)"
	@echo "  make core-debug - Run backend with CLUSTER_DEBUG=1 (tick logs)"
	@echo "  make stress    - Run stress test with 5k VMs (release mode)"

dev:
	@echo "Starting frontend and backend..."
	@$(MAKE) -j2 client core

debug:
	@echo "Starting frontend (with DevTools) and backend (with CLUSTER_DEBUG)..."
	@$(MAKE) -j2 client-debug core-debug

client:
	@echo "Starting frontend (nulltrace-client)..."
	@cd nulltrace-client && npm run tauri dev

client-debug:
	@echo "Starting frontend with DevTools (nulltrace-client)..."
	@cd nulltrace-client && TAURI_OPEN_DEVTOOLS=1 npm run tauri dev

core:
	@echo "Starting backend (nulltrace-core) in GAME MODE..."
	@cd nulltrace-core && cargo run --release --bin cluster

core-debug:
	@echo "Starting backend with CLUSTER_DEBUG (tick logs enabled)..."
	@cd nulltrace-core && CLUSTER_DEBUG=1 cargo run --release --bin cluster

test:
	@echo "Running tests (PostgreSQL required at postgres://nulltrace:nulltrace@localhost:5432/nulltrace)..."
	@cd nulltrace-core && cargo test --bin cluster -- --test-threads=1
	@echo ""
	@echo "Running NTML parser tests..."
	@cd nulltrace-ntml && cargo test
	@echo ""
	@echo "✅ All tests completed!"

test-ntml:
	@echo "Running NTML parser tests..."
	@cd nulltrace-ntml && cargo test
	@echo ""
	@echo "✅ NTML tests completed!"

stress:
	@echo "Running stress test (5000 VMs, 20s, release mode)..."
	@cd nulltrace-core && STRESS_TEST=1 cargo run --release --bin cluster

reset:
	@echo "Resetting Docker compose (down -v, up -d)..."
	@cd nulltrace-core && docker compose down -v && docker compose up -d
	@$(MAKE) dev
