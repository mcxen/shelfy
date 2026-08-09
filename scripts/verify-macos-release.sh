#!/bin/bash

set -euo pipefail

APP="${1:-src-tauri/target/universal-apple-darwin/release/bundle/macos/Shelfy.app}"
EXPECTED_VERSION="${2:-$(node -p 'require("./package.json").version')}"
EXPECTED_ARCHS="${3:-arm64,x86_64}"
PLIST="$APP/Contents/Info.plist"
EXECUTABLE="$APP/Contents/MacOS/shelfy"

test -d "$APP"
test -f "$PLIST"
test -x "$EXECUTABLE"
test "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$PLIST")" = "cc.shelfy.app"
test "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$PLIST")" = "$EXPECTED_VERSION"
test "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$PLIST")" = "shelfy"

ARCHS="$(lipo -archs "$EXECUTABLE")"
IFS=',' read -r -a REQUIRED_ARCHS <<<"$EXPECTED_ARCHS"
for arch in "${REQUIRED_ARCHS[@]}"; do
  grep -qw "$arch" <<<"$ARCHS"
done
codesign --verify --deep --strict "$APP"

echo "Verified Shelfy.app v$EXPECTED_VERSION ($ARCHS)"
