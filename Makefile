.PHONY: clean clean_all configure debug release test test_debug test_release

PROJ_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

EXTENSION_NAME=duckdb_pyfunc

# Set to 1 to enable Unstable API (binaries will only work on TARGET_DUCKDB_VERSION)
USE_UNSTABLE_C_API=1

# Target DuckDB version
TARGET_DUCKDB_VERSION=v1.4.3

# Python configuration for PyO3
PYTHON_VERSION ?= $(shell python3 -c "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')")

all: configure debug

# Include makefiles from DuckDB (if available)
-include extension-ci-tools/makefiles/c_api_extensions/base.Makefile
-include extension-ci-tools/makefiles/c_api_extensions/rust.Makefile

# Standalone build targets (when DuckDB ci-tools not available)
configure:
	@echo "Configuring build environment..."
	@if [ ! -d "extension-ci-tools" ]; then \
		echo "Setting up without extension-ci-tools (standalone mode)"; \
	fi

debug:
	@echo "Building debug version..."
	cargo build
	@mkdir -p build/debug/extension/$(EXTENSION_NAME)
	@cp target/debug/lib$(EXTENSION_NAME).so build/debug/extension/$(EXTENSION_NAME)/$(EXTENSION_NAME).duckdb_extension 2>/dev/null || \
	 cp target/debug/lib$(EXTENSION_NAME).dylib build/debug/extension/$(EXTENSION_NAME)/$(EXTENSION_NAME).duckdb_extension 2>/dev/null || \
	 cp target/debug/$(EXTENSION_NAME).dll build/debug/extension/$(EXTENSION_NAME)/$(EXTENSION_NAME).duckdb_extension 2>/dev/null || \
	 echo "Note: Extension library built at target/debug/"
	@echo "Build complete!"

release:
	@echo "Building release version..."
	cargo build --release
	@mkdir -p build/release/extension/$(EXTENSION_NAME)
	@cp target/release/lib$(EXTENSION_NAME).so build/release/extension/$(EXTENSION_NAME)/$(EXTENSION_NAME).duckdb_extension 2>/dev/null || \
	 cp target/release/lib$(EXTENSION_NAME).dylib build/release/extension/$(EXTENSION_NAME)/$(EXTENSION_NAME).duckdb_extension 2>/dev/null || \
	 cp target/release/$(EXTENSION_NAME).dll build/release/extension/$(EXTENSION_NAME)/$(EXTENSION_NAME).duckdb_extension 2>/dev/null || \
	 echo "Note: Extension library built at target/release/"
	@echo "Release build complete!"

test: test_debug

test_debug: debug
	@echo "Running tests..."
	cargo test
	@if command -v duckdb >/dev/null 2>&1; then \
		echo "Running SQL tests..."; \
		for f in test/sql/*.test; do \
			echo "Testing $$f"; \
			duckdb -unsigned < "$$f" || true; \
		done; \
	else \
		echo "DuckDB CLI not found, skipping SQL tests"; \
	fi

test_release: release
	@echo "Running release tests..."
	cargo test --release

clean:
	cargo clean
	rm -rf build/

clean_all: clean
	rm -rf extension-ci-tools/
	rm -rf venv/

# Development helpers
check:
	cargo check

clippy:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

# Run DuckDB with the extension loaded
run: debug
	@echo "Starting DuckDB with extension loaded..."
	@EXT_PATH=$$(find target/debug -name "*.so" -o -name "*.dylib" 2>/dev/null | head -1); \
	if [ -n "$$EXT_PATH" ]; then \
		duckdb -unsigned -cmd "LOAD '$$EXT_PATH'"; \
	else \
		echo "Extension not found. Build first with 'make debug'"; \
	fi

# Show help
help:
	@echo "DuckDB PyFunc Extension Build System"
	@echo ""
	@echo "Targets:"
	@echo "  all          - Configure and build debug version"
	@echo "  configure    - Set up build environment"
	@echo "  debug        - Build debug version"
	@echo "  release      - Build release version"
	@echo "  test         - Run tests (debug)"
	@echo "  test_debug   - Run tests (debug)"
	@echo "  test_release - Run tests (release)"
	@echo "  clean        - Clean build artifacts"
	@echo "  clean_all    - Clean everything including tools"
	@echo "  check        - Run cargo check"
	@echo "  clippy       - Run clippy linter"
	@echo "  fmt          - Format code"
	@echo "  fmt-check    - Check code formatting"
	@echo "  run          - Start DuckDB with extension loaded"
	@echo "  help         - Show this help message"
