# Makefile for unsigned-gg/ep
SHELL := bash
.SHELLFLAGS := -euo pipefail -c
.DEFAULT_GOAL := help

JOBS ?= 2

.PHONY: help build check test fmt fmt-check clippy smoke clean

help:
	@echo "ep Makefile targets:"
	@echo "  build      - Compile all crates (-j $(JOBS))"
	@echo "  check      - Run cargo check (-j $(JOBS))"
	@echo "  test       - Run test suite (-j $(JOBS))"
	@echo "  fmt        - Format code"
	@echo "  fmt-check  - Check code formatting"
	@echo "  clippy     - Run clippy linter (-D warnings, -j $(JOBS))"
	@echo "  smoke      - Verify ep-domain zero-I/O invariant"
	@echo "  clean      - Clean build artifacts"

build:
	cargo build --workspace -j $(JOBS)

check:
	cargo check --workspace --all-targets -j $(JOBS)

test:
	cargo test --workspace -j $(JOBS)

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace --all-targets -j $(JOBS) -- -D warnings

smoke:
	@if grep -rnE 'std::net|std::fs|tokio|std::process' crates/ep-domain/src; then \
		echo "smoke: ep-domain must not use I/O" >&2; \
		exit 1; \
	fi
	@echo "smoke: ep-domain zero-I/O invariant ok"

clean:
	cargo clean
