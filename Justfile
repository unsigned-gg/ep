set shell := ["bash", "-euo", "pipefail", "-c"]

jobs := "2"

default: help

help:
    @echo "ep Justfile targets:"
    @echo "  build   - Compile all crates (-j {{jobs}})"
    @echo "  check   - Run cargo check (-j {{jobs}})"
    @echo "  test    - Run test suite (-j {{jobs}})"
    @echo "  fmt     - Format code"
    @echo "  clippy  - Run clippy linter"

build:
    cargo build --workspace -j {{jobs}}

check:
    cargo check --workspace --all-targets -j {{jobs}}

test:
    cargo test --workspace -j {{jobs}}

fmt:
    cargo fmt --all

clippy:
    cargo clippy --workspace --all-targets -j {{jobs}} -- -D warnings
