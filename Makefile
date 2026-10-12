SHELL := bash
.SHELLFLAGS := -euo pipefail -c
.DEFAULT_GOAL := help

JOBS ?= 2

.PHONY: help build check test fmt clippy clean

help:
	@echo "ep Makefile targets:"
	@echo "  build   - Compile all crates (-j $(JOBS))"
	@echo "  check   - Run cargo check (-j $(JOBS))"
	@echo "  test    - Run test suite (-j $(JOBS))"
	@echo "  fmt     - Format code"
	@echo "  clippy  - Run clippy linter"
	@echo "  clean   - Clean build artifacts"

build:
	cargo build --workspace -j $(JOBS)

check:
	cargo check --workspace --all-targets -j $(JOBS)

test:
	cargo test --workspace -j $(JOBS)

fmt:
	cargo fmt --all

clippy:
	cargo clippy --workspace --all-targets -j $(JOBS) -- -D warnings

clean:
	cargo clean
