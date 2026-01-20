.PHONY: help build run test check clean sim-setup sim-status sim-create sim-remove

# Default target
help:
	@echo "Termion - Development Commands"
	@echo ""
	@echo "Build:"
	@echo "  make build       Build in debug mode"
	@echo "  make release     Build in release mode"
	@echo "  make run         Build and run TUI"
	@echo ""
	@echo "Quality:"
	@echo "  make check       Run fmt, clippy, and tests"
	@echo "  make test        Run tests"
	@echo "  make fmt         Format code"
	@echo "  make clippy      Run clippy linter"
	@echo ""
	@echo "Simulation (requires MinKNOW + Python):"
	@echo "  make sim-status  Show MinKNOW status and devices"
	@echo "  make sim-create  Create simulated MinION (MS00001)"
	@echo "  make sim-remove  Remove simulated device"
	@echo "  make sim-setup   Full setup with bulk file (set BULK_FILE)"
	@echo ""
	@echo "Maintenance:"
	@echo "  make clean       Remove build artifacts"
	@echo ""
	@echo "Example workflow:"
	@echo "  make sim-create  # Create simulated device"
	@echo "  make run         # Run termion"
	@echo "  # Start a run from MinKNOW GUI"

# =============================================================================
# Build targets
# =============================================================================

build:
	cargo build

release:
	cargo build --release

run:
	cargo run

# =============================================================================
# Quality targets
# =============================================================================

fmt:
	cargo fmt

clippy:
	cargo clippy -- -D warnings

test:
	cargo test

check: fmt clippy test
	@echo "All checks passed!"

# =============================================================================
# Simulation targets
# =============================================================================

# Python venv for minknow_api
DEV_VENV := dev/.venv
SIM_SCRIPT := dev/scripts/sim_manager.py
BULK_FILE ?= 

# Ensure venv exists
$(DEV_VENV)/bin/activate:
	@echo "Creating Python venv for simulation scripts..."
	python3 -m venv $(DEV_VENV)
	$(DEV_VENV)/bin/pip install --quiet minknow_api
	@echo "Done!"

sim-status: $(DEV_VENV)/bin/activate
	@$(DEV_VENV)/bin/python3 $(SIM_SCRIPT) status

sim-create: $(DEV_VENV)/bin/activate
	@$(DEV_VENV)/bin/python3 $(SIM_SCRIPT) create

sim-remove: $(DEV_VENV)/bin/activate
	@$(DEV_VENV)/bin/python3 $(SIM_SCRIPT) remove

sim-playback: $(DEV_VENV)/bin/activate
ifndef BULK_FILE
	@echo "Error: BULK_FILE not set"
	@echo "Usage: make sim-playback BULK_FILE=/path/to/bulk.fast5"
	@exit 1
endif
	@$(DEV_VENV)/bin/python3 $(SIM_SCRIPT) playback $(BULK_FILE)

sim-setup: $(DEV_VENV)/bin/activate
ifndef BULK_FILE
	@echo "Error: BULK_FILE not set"
	@echo "Usage: make sim-setup BULK_FILE=/path/to/bulk.fast5"
	@exit 1
endif
	@$(DEV_VENV)/bin/python3 $(SIM_SCRIPT) setup $(BULK_FILE)

# =============================================================================
# Maintenance
# =============================================================================

clean:
	cargo clean
	rm -rf dev/.venv
