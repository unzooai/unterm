.PHONY: all fmt build check test release-mac clean-release-artifacts

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

# Build, sign, notarize, and upload the macOS Unterm.app for the tag at HEAD.
# Override the keychain notary profile with `make release-mac NOTARY_PROFILE=Foo`.
release-mac:
	bash ci/release-mac.sh

# Remove local packages produced while testing releases. This intentionally
# leaves target/, dist/, and installer/out/ alone because they are broader
# build caches with their own cleanup expectations.
clean-release-artifacts:
	rm -rf Unterm-macos-v*/ Unterm-macos-local-*/ Unterm-macos-2*/
	rm -f Unterm-macos-v*.dmg Unterm-windows-*.zip
	rm -f Unterm-*-x64.msi Unterm-*-arm64.msi
	rm -f Unterm-v*-*.AppImage Unterm-v*-*.AppImage.zsync unterm-v*.deb
	rm -f *.notary.zip
