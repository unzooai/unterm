#!/bin/bash
# Build platform-specific Unterm release artifacts.
# Run from repo root after a release build:
#   cargo build --release -p unterm -p unterm-cli -p unterm-mux -p strip-ansi-escapes
#   ci/deploy.sh
#
# Inputs:
#   TARGET_DIR   cargo target dir (default: target)
#   TAG_NAME     release tag, used in artifact filenames (default: derived from git)
#
# macOS env (optional, for signed/notarized .app):
#   MACOS_TEAM_ID, MACOS_CERT, MACOS_CERT_PW (base64), MACOS_APPLEID, MACOS_APP_PW
set -euo pipefail
set -x

TARGET_DIR=${TARGET_DIR:-${1:-target}}
TAG_NAME=${TAG_NAME:-$(git -c "core.abbrev=8" show -s "--format=%cd-%h" "--date=format:%Y%m%d-%H%M%S")}

if test -z "${SUDO+x}" && hash sudo 2>/dev/null; then
  SUDO="sudo"
fi

if test -e /etc/os-release; then
  . /etc/os-release
fi

case ${OSTYPE:-} in
  darwin*)
    zipdir=Unterm-macos-$TAG_NAME
    zipname=$zipdir.zip
    rm -rf "$zipdir" "$zipname"
    mkdir "$zipdir"
    cp -r assets/macos/Unterm.app "$zipdir/"
    rm -f "$zipdir/Unterm.app/"*.dylib
    mkdir -p "$zipdir/Unterm.app/Contents/MacOS"
    mkdir -p "$zipdir/Unterm.app/Contents/Resources"
    cp -r assets/shell-integration/* "$zipdir/Unterm.app/Contents/Resources"
    cp -r assets/shell-completion "$zipdir/Unterm.app/Contents/Resources"
    tic -xe wezterm -o "$zipdir/Unterm.app/Contents/Resources/terminfo" termwiz/data/wezterm.terminfo

    for bin in unterm unterm-cli unterm-mux strip-ansi-escapes ; do
      if [[ -f "$TARGET_DIR/release/$bin" ]] ; then
        cp "$TARGET_DIR/release/$bin" "$zipdir/Unterm.app/Contents/MacOS/$bin"
      else
        # CI built per-arch (e.g. x86_64-apple-darwin + aarch64-apple-darwin) — fuse with lipo
        lipo "$TARGET_DIR"/*/release/$bin -output "$zipdir/Unterm.app/Contents/MacOS/$bin" -create
      fi
    done

    set +x
    if [ -n "${MACOS_TEAM_ID:-}" ] ; then
      MACOS_PW=$(echo "$MACOS_CERT_PW" | base64 --decode)
      def_keychain=$(eval echo $(security default-keychain -d user))
      security delete-keychain build.keychain || true
      security create-keychain -p "$MACOS_PW" build.keychain
      security default-keychain -d user -s build.keychain
      security unlock-keychain -p "$MACOS_PW" build.keychain
      echo "$MACOS_CERT" | base64 --decode > /tmp/certificate.p12
      security import /tmp/certificate.p12 -k build.keychain -P "$MACOS_PW" -T /usr/bin/codesign
      rm /tmp/certificate.p12
      security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$MACOS_PW" build.keychain
      /usr/bin/codesign --keychain build.keychain --force --options runtime \
        --entitlements ci/macos-entitlement.plist --deep --sign "$MACOS_TEAM_ID" "$zipdir/Unterm.app/"
      security default-keychain -d user -s "$def_keychain"
      security delete-keychain build.keychain || true
    fi
    set -x
    zip -r "$zipname" "$zipdir"
    set +x
    if [ -n "${MACOS_TEAM_ID:-}" ] ; then
      xcrun notarytool submit "$zipname" --wait --team-id "$MACOS_TEAM_ID" \
        --apple-id "$MACOS_APPLEID" --password "$MACOS_APP_PW"
    fi
    set -x
    ;;

  msys|cygwin)
    # Windows: assemble release/ tree expected by installer/Unterm.wxs
    # The MSI itself is built by ci/build-msi.ps1 which expects this layout.
    stagedir=unterm-release-stage/unterm
    rm -rf unterm-release-stage
    mkdir -p "$stagedir"
    cp "$TARGET_DIR/release/unterm.exe" \
       "$TARGET_DIR/release/unterm-cli.exe" \
       "$TARGET_DIR/release/unterm-mux.exe" \
       "$TARGET_DIR/release/strip-ansi-escapes.exe" \
       assets/windows/conhost/conpty.dll \
       assets/windows/conhost/OpenConsole.exe \
       assets/windows/angle/libEGL.dll \
       assets/windows/angle/libGLESv2.dll \
       "$stagedir/"
    mkdir -p "$stagedir/mesa"
    cp "$TARGET_DIR/release/mesa/opengl32.dll" "$stagedir/mesa/" || \
      cp assets/windows/mesa/opengl32.dll "$stagedir/mesa/"
    # Plain zip (MSI is produced by build-msi.ps1)
    zipname=Unterm-windows-$TAG_NAME.zip
    rm -f "$zipname"
    7z a -tzip "$zipname" "$stagedir/"*
    ;;

  linux-gnu|linux)
    # Build a .deb. AppImage is left to ci/build-appimage.sh.
    pkgname=unterm
    arch=$(dpkg-architecture -q DEB_BUILD_ARCH_CPU)
    debroot=pkg/debian
    rm -rf pkg
    mkdir -p "$debroot/DEBIAN"

    # Two flavors of debian/control are needed:
    #
    #   1. `pkg/debian/control` is a *source* package control file (with a
    #      `Source:` stanza). dpkg-shlibdeps requires this format to compute
    #      runtime library deps. We delete it after.
    #
    #   2. `pkg/debian/DEBIAN/control` is a *binary* package control file
    #      (only `Package:` stanza). dpkg-deb requires this format to assemble
    #      the .deb. We append `Depends:` here from shlibdeps' output.
    #
    # Earlier we tried writing one combined file and moving it; dpkg-deb hit
    # "missing 'Package' field" because the Source stanza came first.
    cat > "$debroot/control" <<EOF
Source: $pkgname
Section: utils
Priority: optional
Maintainer: Alex <lixd220@gmail.com>
Homepage: https://github.com/unzooai/unterm
EOF

    cat > "$debroot/DEBIAN/control" <<EOF
Package: $pkgname
Version: ${TAG_NAME#nightly-}
Architecture: $arch
Maintainer: Alex <lixd220@gmail.com>
Section: utils
Priority: optional
Homepage: https://github.com/unzooai/unterm
Description: Unterm terminal emulator
 Unterm is a GPU-accelerated cross-platform terminal emulator
 built on a customized WezTerm engine, with project-aware tabs,
 built-in screenshots, proxy switching, theme switching, and
 a CLI/MCP automation surface.
Provides: x-terminal-emulator
EOF

    install -Dsm755 -t "$debroot/usr/bin" "$TARGET_DIR/release/unterm"
    install -Dsm755 -t "$debroot/usr/bin" "$TARGET_DIR/release/unterm-cli"
    install -Dsm755 -t "$debroot/usr/bin" "$TARGET_DIR/release/unterm-mux"
    install -Dsm755 -t "$debroot/usr/bin" "$TARGET_DIR/release/strip-ansi-escapes"
    install -Dm644 assets/icon/terminal.png "$debroot/usr/share/icons/hicolor/128x128/apps/ai.unzoo.unterm.png"
    install -Dm644 assets/icon/unterm-icon.svg "$debroot/usr/share/icons/hicolor/scalable/apps/ai.unzoo.unterm.svg"
    install -Dm644 assets/unterm.desktop "$debroot/usr/share/applications/ai.unzoo.unterm.desktop"
    install -Dm644 assets/unterm.appdata.xml "$debroot/usr/share/metainfo/ai.unzoo.unterm.appdata.xml"
    install -Dm644 assets/shell-completion/bash "$debroot/usr/share/bash-completion/completions/unterm"
    install -Dm644 assets/shell-completion/zsh "$debroot/usr/share/zsh/functions/Completion/Unix/_unterm"
    install -Dm644 assets/shell-integration/* -t "$debroot/etc/profile.d"

    deps=$(cd pkg && dpkg-shlibdeps -O -e debian/usr/bin/*)
    rm "$debroot/control"  # source-stanza file no longer needed
    echo "$deps" | sed -e 's/shlibs:Depends=/Depends: /' >> "$debroot/DEBIAN/control"

    debname=unterm-$TAG_NAME
    [[ "$arch" != "amd64" ]] && debname="$debname.$arch"
    fakeroot dpkg-deb --build "$debroot" "$debname.deb"
    ;;

  *)
    echo "Unsupported OSTYPE='${OSTYPE:-}'"
    exit 1
    ;;
esac
