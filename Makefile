.DEFAULT_GOAL := help

# Use RELEASE_LEVEL=patch, minor, major, or an explicit version when bumping.
RELEASE_LEVEL ?= release

.PHONY: help build build-release fmt fmt-check lint test check package install smoke tools release publish clean

help:
	@printf '%s\n' \
	  'make build          Build debug binaries' \
	  'make build-release  Build optimized binaries' \
	  'make fmt            Format Rust sources' \
	  'make fmt-check      Check Rust formatting' \
	  'make lint           Run Clippy with warnings denied' \
	  'make test           Run workspace tests' \
	  'make check          Run formatting, Clippy, and tests' \
	  'make package        Build and verify the crates.io package' \
	  'make install        Install rf and rff locally through Cargo' \
	  'make smoke          Build and test the real macOS clipboard' \
	  'make tools          Install the tested cargo-release version' \
	  'make release        Preview a release (RELEASE_LEVEL=patch to bump)' \
	  'make publish        Publish, tag, and push with cargo-release' \
	  'make clean          Remove Cargo build output'

build:
	cargo build --workspace --locked

build-release:
	cargo build --workspace --release --locked

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all --check

lint:
	cargo clippy --workspace --all-targets --locked -- -D warnings

test:
	cargo test --workspace --locked

check: fmt-check lint test

package:
	cargo package --locked

install:
	cargo install --path . --locked

smoke: build-release
	python3 scripts/smoke-macos.py --bin-dir target/release

tools:
	cargo install cargo-release --version 1.1.2 --locked

release:
	cargo release "$(RELEASE_LEVEL)"

publish:
	cargo release "$(RELEASE_LEVEL)" --execute

clean:
	cargo clean
