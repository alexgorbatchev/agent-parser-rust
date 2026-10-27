default:
    @just --list

fmt:
    cargo fmt --package agent-parser

fmt-check:
    cargo fmt --package agent-parser -- --check

lint:
    cargo clippy --all-targets -- -D warnings

test:
    cargo test --all-targets
    cargo test --doc

coverage:
    cargo llvm-cov --all-targets --fail-under-lines 90

check: fmt-check lint test coverage

build:
    cargo build --all-targets

docs:
    cargo doc --no-deps
