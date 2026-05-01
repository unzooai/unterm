.PHONY: all fmt build check test

all: build

test:
	cargo nextest run
	cargo nextest run -p wezterm-escape-parser # no_std by default

check:
	cargo check -p unterm
	cargo check -p unterm-cli
	cargo check -p unterm-mux
	cargo check -p wezterm-escape-parser
	cargo check -p wezterm-cell
	cargo check -p wezterm-surface
	cargo check -p wezterm-ssh

build:
	cargo build $(BUILD_OPTS) -p unterm
	cargo build $(BUILD_OPTS) -p unterm-cli
	cargo build $(BUILD_OPTS) -p unterm-mux
	cargo build $(BUILD_OPTS) -p strip-ansi-escapes

fmt:
	cargo +nightly fmt
