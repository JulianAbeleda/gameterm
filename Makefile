.PHONY: all fmt build check test docs servedocs

all: build

test:
	cargo nextest run
	cargo nextest run -p gameterm-escape-parser # no_std by default

check:
	cargo check
	cargo check -p gameterm-escape-parser
	cargo check -p gameterm-cell
	cargo check -p gameterm-surface
	cargo check -p gameterm-ssh

build:
	cargo build $(BUILD_OPTS) -p gameterm
	cargo build $(BUILD_OPTS) -p gameterm-gui
	cargo build $(BUILD_OPTS) -p gameterm-mux-server
	cargo build $(BUILD_OPTS) -p strip-ansi-escapes

fmt:
	cargo +nightly fmt

docs:
	ci/build-docs.sh

servedocs:
	ci/build-docs.sh serve
